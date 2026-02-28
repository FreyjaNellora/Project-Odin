"""Export PyTorch OdinNNUE weights to .onnue binary format.

The .onnue format must match the Rust implementation in weights.rs exactly:
  Header (48 bytes): magic "ONUE" + version + architecture hash + constants
  Body: quantized weights (FT int16, hidden/output int8/int32)
  Footer: CRC32 checksum (4 bytes)

Usage: python export.py [model.pt] [output.onnue]
"""

import binascii
import struct
import sys

import numpy as np
import torch
from model import OdinNNUE

# Must match Rust constants exactly
ONNUE_MAGIC = b'ONUE'
ONNUE_VERSION = 1
QA = 255
QB = 64
FEATURES = 4480
FT_OUT = 256
HIDDEN = 32


def architecture_hash():
    """Compute the 32-byte architecture hash matching the Rust implementation.

    Uses FNV-1a with 4 different seeds to produce 32 bytes.
    Descriptor: "HalfKP4-4480-256-32-1-4"
    """
    descriptor = f"HalfKP4-{FEATURES}-{FT_OUT}-{HIDDEN}-1-4"
    desc_bytes = descriptor.encode('ascii')
    result = bytearray(32)
    for chunk_idx in range(4):
        h = (0xcbf29ce484222325 + chunk_idx) & 0xFFFFFFFFFFFFFFFF
        for b in desc_bytes:
            h ^= b
            h = (h * 0x00000100000001b3) & 0xFFFFFFFFFFFFFFFF
        result[chunk_idx * 8:chunk_idx * 8 + 8] = h.to_bytes(8, 'little')
    return bytes(result)


def crc32_ieee(data):
    """CRC32 IEEE 802.3, matching the Rust implementation."""
    return binascii.crc32(data) & 0xFFFFFFFF


def quantize_ft(weight, bias, qa=QA):
    """Quantize feature transformer: float -> int16.

    Scale so the maximum weight maps close to QA (255) or int16 max,
    whichever is smaller.
    """
    w_max = max(weight.abs().max().item(), 1e-6)
    scale = min(qa / w_max, 32767.0 / w_max)

    w_q = torch.clamp(torch.round(weight * scale), -32768, 32767).to(torch.int16)
    b_q = torch.clamp(torch.round(bias * scale), -32768, 32767).to(torch.int16)
    return w_q, b_q


def quantize_hidden(weight, bias):
    """Quantize hidden/output layer: float -> int8 weights, int32 biases.

    Biases use the same scale as weights (no QB multiplication).
    The Rust inference adds biases directly as i32 without QB division.
    """
    w_max = max(weight.abs().max().item(), 1e-6)
    scale = min(127.0 / w_max, 127.0)

    w_q = torch.clamp(torch.round(weight * scale), -128, 127).to(torch.int8)
    b_q = torch.round(bias * scale).to(torch.int32)
    return w_q, b_q


def export(model_path, output_path):
    """Export a trained OdinNNUE model to .onnue binary format."""
    model = OdinNNUE()
    model.load_state_dict(torch.load(model_path, map_location='cpu', weights_only=True))
    model.eval()

    buf = bytearray()

    # Header (48 bytes)
    buf += ONNUE_MAGIC                                          # 4 bytes
    buf += struct.pack('<I', ONNUE_VERSION)                     # 4 bytes
    buf += architecture_hash()                                  # 32 bytes
    buf += struct.pack('<I', FEATURES)                          # 4 bytes
    buf += struct.pack('<I', FT_OUT)                            # 4 bytes

    # Feature transformer weights (4 perspectives)
    # Rust layout: [perspective][feature][neuron]
    # PyTorch layout: nn.Linear stores [out, in] = [256, 4480]
    # Must transpose: loop for feat(4480): for neuron(256): read w_q[neuron, feat]
    for p in range(4):
        w = model.ft[p].weight.detach()  # [256, 4480]
        b = model.ft[p].bias.detach()    # [256]
        w_q, b_q = quantize_ft(w, b)
        for feat in range(FEATURES):
            for neuron in range(FT_OUT):
                buf += struct.pack('<h', w_q[neuron, feat].item())
        for neuron in range(FT_OUT):
            buf += struct.pack('<h', b_q[neuron].item())

    # Hidden layer weights
    # Rust layout: [input][neuron] = [1024][32]
    # PyTorch layout: [32, 1024]
    w = model.hidden.weight.detach()   # [32, 1024]
    b = model.hidden.bias.detach()     # [32]
    w_q, b_q = quantize_hidden(w, b)
    for inp in range(4 * FT_OUT):
        for neuron in range(HIDDEN):
            buf += struct.pack('<b', w_q[neuron, inp].item())
    for neuron in range(HIDDEN):
        buf += struct.pack('<i', b_q[neuron].item())

    # BRS head weights
    # Rust layout: [32]
    # PyTorch layout: [1, 32]
    w = model.brs_head.weight.detach()  # [1, 32]
    b = model.brs_head.bias.detach()    # [1]
    w_q, b_q = quantize_hidden(w, b)
    for h in range(HIDDEN):
        buf += struct.pack('<b', w_q[0, h].item())
    buf += struct.pack('<i', b_q[0].item())

    # MCTS head weights
    # Rust layout: [input][output] = [32][4]
    # PyTorch layout: [4, 32]
    w = model.mcts_head.weight.detach()  # [4, 32]
    b = model.mcts_head.bias.detach()    # [4]
    w_q, b_q = quantize_hidden(w, b)
    for h in range(HIDDEN):
        for v in range(4):
            buf += struct.pack('<b', w_q[v, h].item())
    for v in range(4):
        buf += struct.pack('<i', b_q[v].item())

    # CRC32 footer
    checksum = crc32_ieee(bytes(buf))
    buf += struct.pack('<I', checksum)

    with open(output_path, 'wb') as f:
        f.write(buf)

    print(f'Exported {output_path} ({len(buf)} bytes)')
    print(f'CRC32: {checksum:#010x}')


if __name__ == '__main__':
    model_path = sys.argv[1] if len(sys.argv) > 1 else 'best_model.pt'
    output_path = sys.argv[2] if len(sys.argv) > 2 else 'weights_gen0.onnue'
    export(model_path, output_path)
