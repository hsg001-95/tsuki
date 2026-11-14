#!/usr/bin/env python3
"""
Tsuki Model Export & ONNX Verification Pipeline.
Implements Tasks 3.5 & ML-04:
- Exports PyTorch TsukiTCN to ONNX with static shapes (1, 8, 60).
- Verifies model structure with onnx.checker.
- Runs numerical equivalence checks between PyTorch FP32 and ONNX Runtime.
- Exports production model to `models/detector.onnx`.
"""

import argparse
import os
import sys
import numpy as np
import onnx
import onnxruntime as ort
import torch

sys.path.insert(0, os.path.abspath("training"))
from models.tcn import TsukiTCN


def export_and_verify(
    checkpoint_path: str,
    output_onnx_path: str,
    window_size: int = 60,
    num_features: int = 8,
):
    os.makedirs(os.path.dirname(output_onnx_path), exist_ok=True)
    print(f"[*] Initializing model for ONNX export...")

    model = TsukiTCN(num_inputs=num_features, num_channels=[16, 32, 64], kernel_size=3)
    if os.path.exists(checkpoint_path):
        model.load_state_dict(torch.load(checkpoint_path, map_location="cpu"))
        print(f"[*] Loaded weights from: {checkpoint_path}")
    else:
        print(f"[!] Warning: Checkpoint {checkpoint_path} not found. Exporting randomly initialized architecture.")

    model.eval()

    # Dummy input with static shape: (batch_size=1, num_features=8, window_size=60)
    dummy_input = torch.randn(1, num_features, window_size, dtype=torch.float32)

    # Export to ONNX
    print(f"[*] Exporting static ONNX graph to: {output_onnx_path}...")
    torch.onnx.export(
        model,
        dummy_input,
        output_onnx_path,
        export_params=True,
        opset_version=18,
        do_constant_folding=True,
        input_names=["telemetry_window"],
        output_names=["anomaly_prob", "lead_time_sec"],
        dynamic_axes=None,  # Static shapes for deterministic hot-path latency
    )

    # 1. Structural verification
    print("[*] Verifying ONNX model integrity...")
    onnx_model = onnx.load(output_onnx_path)
    onnx.checker.check_model(onnx_model)
    print("[✅] ONNX structural check passed.")

    # 2. Numerical verification
    print("[*] Running numerical equivalence check (PyTorch vs ONNX Runtime)...")
    with torch.no_grad():
        torch_prob, torch_lead = model(dummy_input)
        torch_prob_np = torch_prob.numpy()
        torch_lead_np = torch_lead.numpy()

    ort_session = ort.InferenceSession(output_onnx_path, providers=["CPUExecutionProvider"])
    ort_inputs = {"telemetry_window": dummy_input.numpy()}
    ort_outputs = ort_session.run(None, ort_inputs)
    ort_prob = ort_outputs[0]
    ort_lead = ort_outputs[1]

    prob_diff = np.max(np.abs(torch_prob_np - ort_prob))
    lead_diff = np.max(np.abs(torch_lead_np - ort_lead))

    print(f"Max Probability Discrepancy: {prob_diff:.6f}")
    print(f"Max Lead-Time Discrepancy   : {lead_diff:.6f}")

    assert prob_diff < 1e-4, f"Probability mismatch exceeds tolerance: {prob_diff}"
    assert lead_diff < 1e-4, f"Lead-time mismatch exceeds tolerance: {lead_diff}"
    print("[✅] Numerical equivalence verified: PyTorch and ONNX Runtime outputs match perfectly!")

    file_size_kb = os.path.getsize(output_onnx_path) / 1024.0
    print(f"[✅] Export complete. Final model size: {file_size_kb:.1f} KB")


def main():
    parser = argparse.ArgumentParser(description="Export Tsuki TCN to ONNX")
    parser.add_argument("--checkpoint", type=str, default="training/checkpoints/tcn_best.pt", help="PyTorch checkpoint")
    parser.add_argument("--output", type=str, default="models/detector.onnx", help="Output ONNX path")
    args = parser.parse_args()

    export_and_verify(args.checkpoint, args.output)


if __name__ == "__main__":
    main()
