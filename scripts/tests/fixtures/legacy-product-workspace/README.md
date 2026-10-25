# 旧整 workspace 产品版本映射 fixture

仅用于 schema 1 到 schema 4 的旧产品映射回归，不是运行时维护源。Cargo manifests 与 locks 原样取自 `ecbffed0c053f6f61e70fc0d96e5f34970a88415` 的 `templates/hooks/` 和 `scripts/tests/`；后者以 `consumer-` 文件名保存，测试放回临时工程的原路径。没有生产 Rust 源码，不参与根 workspace。
