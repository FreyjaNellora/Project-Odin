"""OdinDataset — Binary .bin data loader for NNUE training.

Reads the binary training samples produced by the Rust datagen subcommand.
Each sample is a fixed 556-byte record:

  [0..516]   4 feature vectors (4 perspectives x 129 bytes each)
             Per perspective: count:u8 + indices:[u16; 64] (LE, padded)
  [516..518] brs_target: i16 (LE, centipawns)
  [518..534] mcts_targets: [f32; 4] (LE, v1..v4)
  [534..550] game_result: [f32; 4] (LE)
  [550..552] ply: u16 (LE)
  [552..556] game_id: u32 (LE)
"""

import struct
import torch
from torch.utils.data import Dataset

SAMPLE_SIZE = 556  # bytes per sample
PERSPECTIVE_BYTES = 129  # 1 + 64 * 2


class OdinDataset(Dataset):
    def __init__(self, bin_path, num_features=4480):
        with open(bin_path, 'rb') as f:
            self.data = f.read()
        self.num_samples = len(self.data) // SAMPLE_SIZE
        self.num_features = num_features

    def __len__(self):
        return self.num_samples

    def __getitem__(self, idx):
        offset = idx * SAMPLE_SIZE

        # Parse 4 feature vectors → dense [4, num_features] tensor
        features = torch.zeros(4, self.num_features)
        for p in range(4):
            base = offset + p * PERSPECTIVE_BYTES
            count = self.data[base]
            for i in range(count):
                feat_idx = struct.unpack_from('<H', self.data, base + 1 + i * 2)[0]
                if feat_idx < self.num_features:
                    features[p, feat_idx] = 1.0

        target_offset = offset + 516  # after 4 feature vectors

        # BRS target (i16)
        brs_target = struct.unpack_from('<h', self.data, target_offset)[0]

        # MCTS targets (4 x f32)
        mcts_targets = struct.unpack_from('<4f', self.data, target_offset + 2)

        # Game result (4 x f32)
        game_result = struct.unpack_from('<4f', self.data, target_offset + 18)

        return (
            features,
            torch.tensor(brs_target, dtype=torch.float32),
            torch.tensor(mcts_targets, dtype=torch.float32),
            torch.tensor(game_result, dtype=torch.float32),
        )
