.PHONY: run run2 server clean

# 1 instance
run:
ifeq ($(OS),Windows_NT)
	powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run.ps1
else
	mkdir -p app/build
	cd app && pnpm tauri dev
endif

# 2 instances + group server in Docker (Ctrl-C stops everything, data kept in the volume)
run2:
ifeq ($(OS),Windows_NT)
	powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run2.ps1
else
	mkdir -p app/build
	cargo build -p music_app
	docker compose up -d --build
	@trap 'kill 0; docker compose down' INT TERM EXIT; \
	until nc -z 127.0.0.1 8787; do sleep 1; done; \
	(cd app && pnpm tauri dev) & \
	until nc -z 127.0.0.1 1420; do sleep 1; done; \
	MUSIC_APP_PROFILE=b target/debug/music_app & \
	wait
endif

# server only (without Docker)
server:
	cargo run -p music-group-server

# full clean: build artifacts, deps, server db
clean:
	cargo clean
ifeq ($(OS),Windows_NT)
	powershell -NoProfile -Command "Remove-Item -Recurse -Force -ErrorAction SilentlyContinue app/node_modules, app/build, app/.svelte-kit, server.db, server.db-shm, server.db-wal"
else
	rm -rf app/node_modules app/build app/.svelte-kit server.db server.db-shm server.db-wal
endif
