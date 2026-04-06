# TODO

## P0（必须尽快完成）
- [ ] 编写一键初始化脚本（install + download + test）
- [ ] Gemma 3 1B 英文推理测试
- [ ] 接入 OpenClaw 的可行性评估

## P1（重要但不阻塞）
- [ ] 下载测试 Qwen2.5 1.5B（对比中文能力）
- [ ] 不同量化级别性能对比（Q2_K / Q3_K_M / Q4_K_M）
- [ ] 编写基准测试脚本（固定 prompt + 统计 tok/s）

## P2（可延后优化）
- [ ] 测试 Gemma 3 2B（如能找到 GGUF）
- [ ] 探索 llama-server HTTP API 模式
- [ ] 内存优化（ mmap / 降低 n_ctx）

## 已完成归档
- [x] 2026-04-06: 安装 llama-cpp-python，下载 Gemma 3 1B Q4_K_M，首次推理验证
