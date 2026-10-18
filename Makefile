.PHONY: run run2 server clean

# 1 instance
run:
	mkdir -p app/build
	cd app && pnpm tauri dev

# 2 instances + group server in Docker (Ctrl-C stops everything, data kept in the volume)
run2:
	mkdir -p app/build
	cargo build -p music_app
	docker compose up -d --build
	@trap 'kill 0; docker compose down' INT TERM EXIT; \
	until nc -z 127.0.0.1 8787; do sleep 1; done; \
	(cd app && pnpm tauri dev) & \
	until nc -z 127.0.0.1 1420; do sleep 1; done; \
	MUSIC_APP_PROFILE=b target/debug/music_app & \
	wait

# server only (without Docker)
server:
	cargo run -p music-group-server

# full clean: build artifacts, deps, server db
clean:
	cargo clean
	rm -rf app/node_modules app/build app/.svelte-kit server.db server.db-shm server.db-wal
