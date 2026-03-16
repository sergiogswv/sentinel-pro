# 🛡️ SENTINEL PRO

> **Autonomous Real-Time Security & Logic Auditor**

SENTINEL is a high-performance monitoring agent written in **Rust**. It watches your project's filesystem for changes and provides immediate architectural audits, regression detection, and interactive AI feedback.

---

## 🚀 Key Responsibilities

- **Real-Time Monitoring:** Native filesystem watching with debouncing.
- **Architectural Guard:** Validates code against structural rules in real-time.
- **Business Logic Guard:** Detects potential regressions by comparing changes with Git history.
- **Interactive Prompts:** Requests user confirmation via the SKRYMIR Dashboard before proceeding with deep audits.

---

## ⚙️ Installation

```bash
# 1. Clone
git clone https://github.com/sergiogswv/sentinel-pro.git
cd sentinel-pro

# 2. Build (Requires Rust & Cargo)
cargo build --release

# 3. Run
./target/release/sentinel serve
```

---

## 📜 License
© 2026 Sergio - SKRYMIR Intelligence Command.
