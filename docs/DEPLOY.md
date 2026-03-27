# Развёртывание, тестирование и запуск

## 1. Требования
- Ubuntu 20.04+ (включая Xfce/LXQt)
- Rust toolchain (stable)
- build-essential, pkg-config

## 2. Установка Rust
```bash
curl https://sh.rustup.rs -sSf | sh
source "$HOME/.cargo/env"
rustup default stable
```

## 3. Получение проекта
```bash
git clone <repo_url>
cd church-warden
```

## 4. Сборка
```bash
cargo build --release
```

## 5. Тестирование
```bash
cargo test
```

## 6. Запуск
```bash
cargo run
```

После запуска создаётся/используется файл `church_warden.db` в рабочем каталоге.
