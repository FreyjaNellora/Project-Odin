"""Stage 15 Python acceptance tests — NNUE training pipeline.

T6:  test_model_forward_shape
T7:  test_dataset_loading
T8:  test_loss_computation
T9:  test_export_magic
T10: test_export_architecture_hash
T11: test_export_roundtrip
T12: test_training_loss_decreases

Run: python -m pytest test_pipeline.py -v
"""

import os
import struct
import tempfile

import torch

from model import OdinNNUE
from dataset import OdinDataset, SAMPLE_SIZE
from train import compute_loss
from export import architecture_hash, crc32_ieee, export


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def create_synthetic_bin(path, num_samples=100, num_features=4480):
    """Create a synthetic .bin file with random but valid training samples."""
    import random
    random.seed(42)

    with open(path, 'wb') as f:
        for sample_idx in range(num_samples):
            # 4 perspectives
            for p in range(4):
                count = random.randint(20, 40)
                f.write(struct.pack('B', count))
                indices = random.sample(range(num_features), count)
                for idx in indices:
                    f.write(struct.pack('<H', idx))
                # Pad remaining indices to 64
                for _ in range(64 - count):
                    f.write(struct.pack('<H', 0))

            # BRS target
            brs = random.randint(-5000, 5000)
            f.write(struct.pack('<h', brs))

            # MCTS targets
            for _ in range(4):
                f.write(struct.pack('<f', random.random()))

            # Game result
            winner = random.randint(0, 3)
            for i in range(4):
                f.write(struct.pack('<f', 1.0 if i == winner else 0.0))

            # Metadata
            f.write(struct.pack('<H', sample_idx))  # ply
            f.write(struct.pack('<I', sample_idx // 10))  # game_id


# ---------------------------------------------------------------------------
# T6: Model forward pass shape
# ---------------------------------------------------------------------------

def test_model_forward_shape():
    model = OdinNNUE()
    batch_size = 2
    features = torch.randn(batch_size, 4, 4480)

    brs_out, mcts_out = model(features)

    assert brs_out.shape == (batch_size, 1), f"BRS shape: {brs_out.shape}"
    assert mcts_out.shape == (batch_size, 4), f"MCTS shape: {mcts_out.shape}"


def test_model_forward_deterministic():
    model = OdinNNUE()
    model.eval()
    features = torch.randn(1, 4, 4480)

    with torch.no_grad():
        out1 = model(features)
        out2 = model(features)

    assert torch.allclose(out1[0], out2[0]), "BRS output not deterministic"
    assert torch.allclose(out1[1], out2[1]), "MCTS output not deterministic"


# ---------------------------------------------------------------------------
# T7: Dataset loading
# ---------------------------------------------------------------------------

def test_dataset_loading():
    with tempfile.NamedTemporaryFile(suffix='.bin', delete=False) as f:
        tmp_path = f.name

    try:
        create_synthetic_bin(tmp_path, num_samples=10)
        dataset = OdinDataset(tmp_path)

        assert len(dataset) == 10, f"Expected 10 samples, got {len(dataset)}"

        features, brs_target, mcts_targets, game_result = dataset[0]

        assert features.shape == (4, 4480), f"Feature shape: {features.shape}"
        assert brs_target.shape == (), f"BRS shape: {brs_target.shape}"
        assert mcts_targets.shape == (4,), f"MCTS shape: {mcts_targets.shape}"
        assert game_result.shape == (4,), f"Result shape: {game_result.shape}"

        # Features should be sparse (mostly zeros, some ones)
        nonzero = (features > 0).sum().item()
        assert 20 <= nonzero <= 200, f"Expected 20-200 nonzero features, got {nonzero}"

        # Game result should sum to 1.0
        result_sum = game_result.sum().item()
        assert abs(result_sum - 1.0) < 0.01, f"Game result sum: {result_sum}"
    finally:
        os.unlink(tmp_path)


# ---------------------------------------------------------------------------
# T8: Loss computation
# ---------------------------------------------------------------------------

def test_loss_computation():
    model = OdinNNUE()
    batch_size = 4
    features = torch.randn(batch_size, 4, 4480)
    brs_target = torch.randn(batch_size)
    mcts_target = torch.rand(batch_size, 4)
    game_result = torch.zeros(batch_size, 4)
    game_result[:, 0] = 1.0  # Red always wins

    total, brs_loss, mcts_loss, result_loss = compute_loss(
        model, features, brs_target, mcts_target, game_result
    )

    assert not torch.isnan(total), "Total loss is NaN"
    assert not torch.isinf(total), "Total loss is Inf"
    assert total.item() > 0, "Total loss should be positive"

    assert not torch.isnan(brs_loss), "BRS loss is NaN"
    assert not torch.isnan(mcts_loss), "MCTS loss is NaN"
    assert not torch.isnan(result_loss), "Result loss is NaN"


# ---------------------------------------------------------------------------
# T9: Export magic bytes
# ---------------------------------------------------------------------------

def test_export_magic():
    model = OdinNNUE()

    with tempfile.NamedTemporaryFile(suffix='.pt', delete=False) as f:
        pt_path = f.name
    with tempfile.NamedTemporaryFile(suffix='.onnue', delete=False) as f:
        onnue_path = f.name

    try:
        torch.save(model.state_dict(), pt_path)
        export(pt_path, onnue_path)

        with open(onnue_path, 'rb') as f:
            magic = f.read(4)

        assert magic == b'ONUE', f"Magic bytes: {magic}"
    finally:
        os.unlink(pt_path)
        os.unlink(onnue_path)


# ---------------------------------------------------------------------------
# T10: Architecture hash matches Rust implementation
# ---------------------------------------------------------------------------

def test_export_architecture_hash():
    """Verify Python architecture_hash() produces the expected 32 bytes.

    The expected value is computed by running the same FNV-1a algorithm
    over "HalfKP4-4480-256-32-1-4" with seeds 0,1,2,3.
    """
    h = architecture_hash()
    assert len(h) == 32, f"Hash length: {len(h)}"

    # Verify it's deterministic
    h2 = architecture_hash()
    assert h == h2, "Architecture hash not deterministic"

    # Verify the descriptor matches what Rust uses
    descriptor = "HalfKP4-4480-256-32-1-4"

    # Manually compute chunk 0 to cross-check
    fnv_offset = 0xcbf29ce484222325
    fnv_prime = 0x00000100000001b3
    h_check = fnv_offset  # seed 0
    for b in descriptor.encode('ascii'):
        h_check ^= b
        h_check = (h_check * fnv_prime) & 0xFFFFFFFFFFFFFFFF
    expected_chunk0 = h_check.to_bytes(8, 'little')
    assert h[:8] == expected_chunk0, (
        f"Chunk 0 mismatch: got {h[:8].hex()}, expected {expected_chunk0.hex()}"
    )


# ---------------------------------------------------------------------------
# T11: Export roundtrip — export → verify header fields
# ---------------------------------------------------------------------------

def test_export_roundtrip():
    model = OdinNNUE()

    with tempfile.NamedTemporaryFile(suffix='.pt', delete=False) as f:
        pt_path = f.name
    with tempfile.NamedTemporaryFile(suffix='.onnue', delete=False) as f:
        onnue_path = f.name

    try:
        torch.save(model.state_dict(), pt_path)
        export(pt_path, onnue_path)

        with open(onnue_path, 'rb') as f:
            data = f.read()

        # Verify header
        assert data[:4] == b'ONUE', "Magic mismatch"
        version = struct.unpack_from('<I', data, 4)[0]
        assert version == 1, f"Version: {version}"

        stored_hash = data[8:40]
        expected_hash = architecture_hash()
        assert stored_hash == expected_hash, "Architecture hash mismatch"

        features = struct.unpack_from('<I', data, 40)[0]
        assert features == 4480, f"Features: {features}"

        ft_out = struct.unpack_from('<I', data, 44)[0]
        assert ft_out == 256, f"FT_OUT: {ft_out}"

        # Verify CRC32
        payload = data[:-4]
        stored_crc = struct.unpack_from('<I', data, len(data) - 4)[0]
        computed_crc = crc32_ieee(payload)
        assert stored_crc == computed_crc, (
            f"CRC32 mismatch: stored={stored_crc:#010x}, computed={computed_crc:#010x}"
        )

        # Verify file size is reasonable (header + body + footer)
        # Body: FT weights + biases + hidden + BRS + MCTS
        expected_body = (
            4 * 4480 * 256 * 2  # FT weights (i16)
            + 4 * 256 * 2       # FT biases (i16)
            + 1024 * 32         # hidden weights (i8)
            + 32 * 4            # hidden biases (i32)
            + 32                # BRS weights (i8)
            + 4                 # BRS bias (i32)
            + 32 * 4            # MCTS weights (i8)
            + 4 * 4             # MCTS biases (i32)
        )
        expected_total = 48 + expected_body + 4
        assert len(data) == expected_total, (
            f"File size: {len(data)}, expected: {expected_total}"
        )
    finally:
        os.unlink(pt_path)
        os.unlink(onnue_path)


# ---------------------------------------------------------------------------
# T12: Training loss decreases over 5 epochs
# ---------------------------------------------------------------------------

def test_training_loss_decreases():
    torch.manual_seed(42)

    with tempfile.NamedTemporaryFile(suffix='.bin', delete=False) as f:
        tmp_path = f.name

    try:
        create_synthetic_bin(tmp_path, num_samples=500)
        dataset = OdinDataset(tmp_path)

        from torch.utils.data import DataLoader

        loader = DataLoader(dataset, batch_size=64, shuffle=True)

        model = OdinNNUE()
        optimizer = torch.optim.Adam(model.parameters(), lr=0.001)

        losses = []
        for epoch in range(10):
            model.train()
            epoch_loss = 0.0
            batches = 0
            for features, brs_target, mcts_target, game_result in loader:
                loss, _, _, _ = compute_loss(
                    model, features, brs_target, mcts_target, game_result
                )
                optimizer.zero_grad()
                loss.backward()
                optimizer.step()
                epoch_loss += loss.item()
                batches += 1
            avg_loss = epoch_loss / max(batches, 1)
            losses.append(avg_loss)

        # Best loss across all epochs should be less than initial loss
        best_loss = min(losses)
        assert best_loss < losses[0], (
            f"Loss did not decrease: first={losses[0]:.6f}, best={best_loss:.6f}, "
            f"all={[f'{l:.6f}' for l in losses]}"
        )
    finally:
        os.unlink(tmp_path)
