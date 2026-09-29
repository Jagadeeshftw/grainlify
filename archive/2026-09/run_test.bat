@echo off
cd /d "C:\Users\HP\Documents\Code\Drips\grainlify\contracts\grainlify-core"
set GRAINLIFY_PRINT_SERIALIZATION_GOLDENS=1
cargo test --lib serialization_compatibility_public_types_and_events -- --nocapture
