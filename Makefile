.PHONY: run run2 server clean

# 1 instance
run:
	mkdir -p build
	pnpm tauri dev

# 2 instances + group server (Ctrl-C stops everything)
run2:
	mkdir -p build
	cargo build -p music_app -p music-group-server
	@trap 'kill 0' INT TERM EXIT; \
	target/debug/music-group-server & \
	pnpm tauri dev & \
	until nc -z 127.0.0.1 1420; do sleep 1; done; \
	MUSIC_APP_PROFILE=b target/debug/music_app & \
	wait

# server only
server:
	cargo run -p music-group-server

# full clean: build artifacts, deps, server db
clean:
	cargo clean
	rm -rf node_modules build .svelte-kit server.db server.db-shm server.db-wal
