import os
import sys
import numpy as np
import onnxruntime as ort
import pytest
import torch

sys.path.insert(0, os.path.abspath("training"))
sys.path.insert(0, os.path.abspath("bench"))

from calibrate import TemperatureScaler
from models.tcn import TsukiTCN
from train_detector import create_sliding_windows


def test_tcn_forward_shape():
    model = TsukiTCN(num_inputs=8, num_channels=[16, 32], kernel_size=3)
    model.eval()

    # Input: (batch=2, features=8, window=60)
    x = torch.randn(2, 8, 60)
    prob, lead_time = model(x)

    assert prob.shape == (2, 1)
    assert lead_time.shape == (2, 1)
    assert (prob >= 0.0).all() and (prob <= 1.0).all()
    assert (lead_time >= 0.0).all()


def test_sliding_window_construction():
    X_raw = np.ones((100, 8), dtype=np.float32)
    y_raw = np.zeros(100, dtype=np.float32)

    windows, labels = create_sliding_windows(X_raw, y_raw, window_size=60)
    assert windows.shape == (41, 8, 60)
    assert labels.shape == (41,)


def test_temperature_scaling_calibration():
    logits = np.array([-2.0, 0.0, 2.0])
    scaler = TemperatureScaler(temperature=2.0)
    probs = scaler.transform(logits)

    assert len(probs) == 3
    assert probs[1] == 0.5  # sigmoid(0) is always 0.5
    assert probs[0] < probs[1] < probs[2]


def test_exported_onnx_model():
    onnx_path = "models/detector.onnx"
    assert os.path.exists(onnx_path), "Exported ONNX model should exist at models/detector.onnx"

    session = ort.InferenceSession(onnx_path, providers=["CPUExecutionProvider"])
    dummy = np.random.randn(1, 8, 60).astype(np.float32)

    outputs = session.run(None, {"telemetry_window": dummy})
    assert len(outputs) == 2
    prob, lead_time = outputs[0], outputs[1]

    assert prob.shape == (1, 1)
    assert lead_time.shape == (1, 1)
    assert 0.0 <= prob[0, 0] <= 1.0
