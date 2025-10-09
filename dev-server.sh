#!/bin/bash
# Development server startup script with Google credentials
export GOOGLE_APPLICATION_CREDENTIALS="$(pwd)/google-credentials.json"
cargo leptos watch
