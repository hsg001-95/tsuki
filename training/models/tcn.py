"""
Tsuki Early-Warning Detector: Temporal Convolutional Network (TCN).
Implemented with standard PyTorch layers (nn.Conv1d, nn.Sequential).
Evaluates multivariate host telemetry time-series windows (PSI, vmstat, cgroups)
to predict Linux freeze states with useful lead-time (>= 10s).
Compatible with ONNX static export and INT8/FP16 quantization.
"""

import torch
import torch.nn as nn
import torch.nn.functional as F


class TemporalBlock(nn.Module):
    def __init__(
        self,
        n_inputs: int,
        n_outputs: int,
        kernel_size: int,
        stride: int,
        dilation: int,
        dropout: float = 0.1,
    ):
        super().__init__()
        self.padding = (kernel_size - 1) * dilation // 2

        self.conv1 = nn.Conv1d(
            n_inputs,
            n_outputs,
            kernel_size,
            stride=stride,
            padding=self.padding,
            dilation=dilation,
        )
        self.bn1 = nn.BatchNorm1d(n_outputs)
        self.relu1 = nn.ReLU()
        self.dropout1 = nn.Dropout(dropout)

        self.conv2 = nn.Conv1d(
            n_outputs,
            n_outputs,
            kernel_size,
            stride=stride,
            padding=self.padding,
            dilation=dilation,
        )
        self.bn2 = nn.BatchNorm1d(n_outputs)
        self.relu2 = nn.ReLU()
        self.dropout2 = nn.Dropout(dropout)

        self.downsample = (
            nn.Conv1d(n_inputs, n_outputs, 1) if n_inputs != n_outputs else None
        )

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        res = x if self.downsample is None else self.downsample(x)

        out = self.conv1(x)
        out = self.bn1(out)
        out = self.relu1(out)
        out = self.dropout1(out)

        out = self.conv2(out)
        out = self.bn2(out)

        # Match shapes if slight padding discrepancy occurs
        if out.shape[-1] != res.shape[-1]:
            diff = res.shape[-1] - out.shape[-1]
            out = F.pad(out, (0, diff))

        out = self.relu2(out + res)
        out = self.dropout2(out)
        return out


class TsukiTCN(nn.Module):
    def __init__(
        self,
        num_inputs: int = 8,
        num_channels: list = None,
        kernel_size: int = 3,
        dropout: float = 0.1,
    ):
        super().__init__()
        if num_channels is None:
            num_channels = [16, 32, 64]

        layers = []
        num_levels = len(num_channels)
        for i in range(num_levels):
            dilation_size = 2**i
            in_channels = num_inputs if i == 0 else num_channels[i - 1]
            out_channels = num_channels[i]
            layers.append(
                TemporalBlock(
                    in_channels,
                    out_channels,
                    kernel_size,
                    stride=1,
                    dilation=dilation_size,
                    dropout=dropout,
                )
            )

        self.network = nn.Sequential(*layers)
        self.adaptive_pool = nn.AdaptiveAvgPool1d(1)

        # Head 1: Early-warning binary classification (freeze within horizon)
        self.classifier = nn.Sequential(
            nn.Linear(num_channels[-1], 32),
            nn.ReLU(),
            nn.Linear(32, 1),
        )

        # Head 2: Auxiliary lead-time regression (seconds remaining until stall)
        self.lead_time_regressor = nn.Sequential(
            nn.Linear(num_channels[-1], 16),
            nn.ReLU(),
            nn.Linear(16, 1),
        )

    def forward(self, x: torch.Tensor):
        """
        x shape: (batch_size, num_features, window_size), e.g. (B, 8, 60)
        Returns:
            prob: (batch_size, 1) probability of freeze in horizon
            lead_time: (batch_size, 1) predicted seconds until freeze
        """
        features = self.network(x)
        pooled = self.adaptive_pool(features).squeeze(-1)

        logit = self.classifier(pooled)
        prob = torch.sigmoid(logit)

        lead_time = F.relu(self.lead_time_regressor(pooled))
        return prob, lead_time
