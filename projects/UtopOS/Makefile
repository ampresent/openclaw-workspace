.PHONY: build rpm install clean test help

NAME    := utopos
VERSION := $(shell grep '^Version:' packaging/rpm/utopos.spec | awk '{print $$2}')

help: ## 显示帮助
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'

build: ## 构建 Rust 二进制
	cd evo && cargo build --release

rpm: ## 构建 RPM 包
	./build-rpm.sh --local

install: rpm ## 构建并安装 RPM
	./build-rpm.sh --local --install

test: ## 运行测试
	cd evo && cargo test

clean: ## 清理构建产物
	cd evo && cargo clean
	rm -rf ~/rpmbuild/BUILD/$(NAME)-$(VERSION)
