"""OdinNNUE — PyTorch network architecture for 4-player chess NNUE.

Architecture matches Stage 14 Rust inference exactly:
  Input: 4,480 sparse features per perspective (HalfKP-4)
  Feature Transformer: 4,480 -> 256 (SCReLU) x4 perspectives
  Concatenate: 4 x 256 = 1024
  Hidden: 1024 -> 32 (ClippedReLU)
  Dual Output Heads:
    - BRS Head: 32 -> 1 (centipawn scalar)
    - MCTS Head: 32 -> 4 (per-player sigmoid values)
"""

import torch
import torch.nn as nn


class SCReLU(nn.Module):
    """Squared Clipped ReLU: clamp(x, 0, QA)^2"""

    def __init__(self, qa=255.0):
        super().__init__()
        self.qa = qa

    def forward(self, x):
        return torch.clamp(x, 0.0, self.qa) ** 2


class OdinNNUE(nn.Module):
    def __init__(self, num_features=4480, ft_out=256, hidden=32):
        super().__init__()
        self.num_features = num_features
        self.ft_out = ft_out
        self.qa = 255.0

        # 4 separate feature transformers (one per perspective)
        self.ft = nn.ModuleList([
            nn.Linear(num_features, ft_out) for _ in range(4)
        ])
        self.screlu = SCReLU(self.qa)

        # Hidden layer: 4*256 = 1024 -> 32
        self.hidden = nn.Linear(4 * ft_out, hidden)

        # Dual output heads
        self.brs_head = nn.Linear(hidden, 1)       # BRS scalar
        self.mcts_head = nn.Linear(hidden, 4)      # MCTS 4-player

    def forward(self, features):
        """
        features: [batch, 4, 4480] dense tensor (sparse features expanded)
        Returns: (brs_out, mcts_out)
          brs_out: [batch, 1] raw centipawn prediction
          mcts_out: [batch, 4] raw logits (sigmoid applied in loss)
        """
        # Feature transformer per perspective
        ft_outs = []
        for p in range(4):
            ft_outs.append(self.screlu(self.ft[p](features[:, p, :])))
        # Concatenate: [batch, 1024]
        concat = torch.cat(ft_outs, dim=1)
        # Normalize after SCReLU squaring (divide by QA to keep values reasonable)
        # This mirrors Rust inference: activated[i] / qa
        concat = concat / self.qa

        # Hidden layer + ReLU
        h = torch.relu(self.hidden(concat))

        # Output heads
        brs_out = self.brs_head(h)
        mcts_out = self.mcts_head(h)

        return brs_out, mcts_out
