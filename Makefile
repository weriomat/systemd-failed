flame: 
	CARGO_PROFILE_RELEASE_DEBUG=true cargo-flamegraph flamegraph --bin systemd-failed
	# CARGO_PROFILE_RELEASE_DEBUG=true cargo-flamegraph flamegraph --bin filestat -- --path /home
	firefox flamegraph.svg
bench: 
	hyperfine --show-output --warmup 5 --min-runs 10 "./target/release/filestat --path /home/marts"
doc:
	firefox "$(rustc --print sysroot)/share/doc/rust/html/index.html"
	cargo doc --open
c: 
	cargo clippy
