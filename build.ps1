Set-Location "F:\Solana DeFi Vault"
$env:PATH="C:\Users\Admin\solana-release\bin;C:\Users\Admin\.cargo\bin;" + $env:PATH

# Use default system Rust (1.92) which is compatible with newer crates
# Solana 2.0.21 toolchain should handle the SBF compilation

anchor build
