// Engine lifecycle and IPC bridge.
// Manages spawning, command sending, and stdout event listening.

import { useState, useEffect, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { EngineMessage } from '../types/protocol';
import { parseEngineOutput } from '../lib/protocol-parser';

/** Maximum number of raw log lines to keep. */
const MAX_LOG_LINES = 1000;
/** Lines to drop when log exceeds maximum. */
const DROP_LINES = 200;

export interface UseEngineResult {
  isConnected: boolean;
  engineName: string;
  rawLog: string[];
  lastMessage: EngineMessage | null;
  spawnEngine: () => Promise<void>;
  sendCommand: (cmd: string) => Promise<void>;
  killEngine: () => Promise<void>;
  /** Register a callback for specific message types. */
  onMessage: (handler: (msg: EngineMessage) => void) => void;
}

export function useEngine(): UseEngineResult {
  const [isConnected, setIsConnected] = useState(false);
  const [engineName, setEngineName] = useState('');
  const [rawLog, setRawLog] = useState<string[]>([]);
  const [lastMessage, setLastMessage] = useState<EngineMessage | null>(null);
  const messageHandlerRef = useRef<((msg: EngineMessage) => void) | null>(null);

  // Listen for engine output events
  useEffect(() => {
    const unlisten = listen<string>('engine-output', (event) => {
      const line = event.payload;

      // Add to raw log
      setRawLog((prev) => {
        const next = [...prev, line];
        if (next.length > MAX_LOG_LINES) {
          return next.slice(DROP_LINES);
        }
        return next;
      });

      // Parse and dispatch
      const msg = parseEngineOutput(line);
      setLastMessage(msg);

      // Track connection state from protocol messages
      if (msg.type === 'id' && msg.key === 'name') {
        setEngineName(msg.value);
      }

      // Notify handler
      if (messageHandlerRef.current) {
        messageHandlerRef.current(msg);
      }
    });

    const unlistenExit = listen<number>('engine-exit', () => {
      setIsConnected(false);
    });

    return () => {
      unlisten.then((fn) => fn());
      unlistenExit.then((fn) => fn());
    };
  }, []);

  const spawnEngine = useCallback(async () => {
    try {
      await invoke('spawn_engine');
      setIsConnected(true);
      // Send initialization sequence
      await invoke('send_command', { cmd: 'odin' });
      await invoke('send_command', { cmd: 'isready' });
    } catch (e) {
      setIsConnected(false);
      throw e;
    }
  }, []);

  const sendCommand = useCallback(async (cmd: string) => {
    await invoke('send_command', { cmd });
  }, []);

  const killEngine = useCallback(async () => {
    try {
      await invoke('kill_engine');
    } finally {
      setIsConnected(false);
    }
  }, []);

  const onMessage = useCallback((handler: (msg: EngineMessage) => void) => {
    messageHandlerRef.current = handler;
  }, []);

  return {
    isConnected,
    engineName,
    rawLog,
    lastMessage,
    spawnEngine,
    sendCommand,
    killEngine,
    onMessage,
  };
}
