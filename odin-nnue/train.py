"""Training loop for OdinNNUE — multi-task loss with early stopping.

Loss: LAMBDA_BRS * MSE(brs) + LAMBDA_MCTS * MSE(mcts) + LAMBDA_RESULT * MSE(result)

BRS loss: MSE between predicted/target centipawns (both normalized by OUTPUT_SCALE=400).
MCTS loss: MSE between sigmoid(pred/SIGMOID_K) and blended target (70% search + 30% result).
Result loss: MSE between sigmoid(pred/SIGMOID_K) and game result.

Usage: python train.py [path_to_bin] [output_model_path]
"""

import sys
import torch
import torch.nn as nn
from torch.utils.data import DataLoader, random_split
from model import OdinNNUE
from dataset import OdinDataset

# Hyperparameters
BATCH_SIZE = 4096
LR = 0.01
EPOCHS = 20
LAMBDA_BRS = 1.0
LAMBDA_MCTS = 0.5
LAMBDA_RESULT = 0.25
OUTPUT_SCALE = 400.0
SIGMOID_K = 4000.0


def compute_loss(model, features, brs_target, mcts_target, game_result):
    """Compute multi-task loss. Returns (total_loss, brs_loss, mcts_loss, result_loss)."""
    brs_pred, mcts_pred = model(features)

    # BRS loss: MSE in normalized centipawn scale
    brs_loss = nn.functional.mse_loss(
        brs_pred.squeeze() / OUTPUT_SCALE,
        brs_target / OUTPUT_SCALE
    )

    # MCTS loss: MSE between sigmoid predictions and blended target
    # Blend: 70% search value + 30% game result
    mcts_blended = 0.7 * mcts_target + 0.3 * game_result
    mcts_pred_sigmoid = torch.sigmoid(mcts_pred / SIGMOID_K)
    mcts_loss = nn.functional.mse_loss(mcts_pred_sigmoid, mcts_blended)

    # Game result loss: how well does the model predict game outcomes?
    result_pred = torch.sigmoid(mcts_pred / SIGMOID_K)
    result_loss = nn.functional.mse_loss(result_pred, game_result)

    total = LAMBDA_BRS * brs_loss + LAMBDA_MCTS * mcts_loss + LAMBDA_RESULT * result_loss
    return total, brs_loss, mcts_loss, result_loss


def train(bin_path='training_data_gen0.bin', model_path='best_model.pt'):
    # Auto-detect platform for DataLoader workers
    num_workers = 0 if sys.platform == 'win32' else 4

    dataset = OdinDataset(bin_path)
    print(f'Dataset: {len(dataset)} samples from {bin_path}')

    if len(dataset) < 10:
        print('ERROR: Too few samples to train. Need at least 10.')
        sys.exit(1)

    train_size = int(0.9 * len(dataset))
    val_size = len(dataset) - train_size
    train_set, val_set = random_split(dataset, [train_size, val_size])

    train_loader = DataLoader(
        train_set, batch_size=BATCH_SIZE, shuffle=True, num_workers=num_workers
    )
    val_loader = DataLoader(
        val_set, batch_size=BATCH_SIZE, shuffle=False, num_workers=num_workers
    )

    model = OdinNNUE()
    optimizer = torch.optim.Adam(model.parameters(), lr=LR)
    scheduler = torch.optim.lr_scheduler.StepLR(optimizer, step_size=5, gamma=0.5)

    best_val_loss = float('inf')
    patience = 5
    patience_counter = 0

    for epoch in range(EPOCHS):
        model.train()
        total_loss = 0
        for features, brs_target, mcts_target, game_result in train_loader:
            loss, _, _, _ = compute_loss(model, features, brs_target, mcts_target, game_result)

            optimizer.zero_grad()
            loss.backward()
            optimizer.step()

            total_loss += loss.item()

        # Validation
        model.eval()
        val_loss = 0
        with torch.no_grad():
            for features, brs_target, mcts_target, game_result in val_loader:
                loss, _, _, _ = compute_loss(model, features, brs_target, mcts_target, game_result)
                val_loss += loss.item()

        avg_train = total_loss / max(len(train_loader), 1)
        avg_val = val_loss / max(len(val_loader), 1)
        lr = scheduler.get_last_lr()[0]
        print(f'Epoch {epoch+1}/{EPOCHS}  train_loss={avg_train:.6f}  val_loss={avg_val:.6f}  lr={lr:.6f}')

        # Early stopping
        if avg_val < best_val_loss:
            best_val_loss = avg_val
            patience_counter = 0
            torch.save(model.state_dict(), model_path)
        else:
            patience_counter += 1
            if patience_counter >= patience:
                print(f'Early stopping at epoch {epoch+1}')
                break

        scheduler.step()

    print(f'Best validation loss: {best_val_loss:.6f}')
    print(f'Model saved to {model_path}')


if __name__ == '__main__':
    bin_path = sys.argv[1] if len(sys.argv) > 1 else 'training_data_gen0.bin'
    model_path = sys.argv[2] if len(sys.argv) > 2 else 'best_model.pt'
    train(bin_path, model_path)
