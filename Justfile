doc:
    cargo doc --document-private-items

doc-open:
    cargo doc --document-private-items --open

watch:
    cargo watch -x 'clippy --all --all-features'
