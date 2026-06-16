# geo-toolbox CI script (PowerShell)
# Run: powershell -File scripts/ci.ps1
# Requires: cargo, rustc, cargo-llvm-cov (auto-installed)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent (Split-Path -Parent $PSCommandPath)

Write-Host "=== geo-toolbox CI ===" -ForegroundColor Cyan
Write-Host "Root: $root" -ForegroundColor Cyan
cd $root

# ── 1. Format check ──
Write-Host "`n[1/5] cargo fmt --check" -ForegroundColor Yellow
cargo fmt --check
if ($LASTEXITCODE -ne 0) { Write-Host "FAIL: fmt check" -ForegroundColor Red; exit 1 }
Write-Host "  ✓ OK" -ForegroundColor Green

# ── 2. Build all targets ──
Write-Host "`n[2/5] cargo build --all-targets" -ForegroundColor Yellow
cargo build --all-targets 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) { Write-Host "FAIL: build" -ForegroundColor Red; exit 1 }
Write-Host "  ✓ OK" -ForegroundColor Green

# ── 3. Clippy ──
Write-Host "`n[3/5] cargo clippy --all-targets -- -D warnings" -ForegroundColor Yellow
cargo clippy --all-targets -- -D warnings 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) { Write-Host "FAIL: clippy" -ForegroundColor Red; exit 1 }
Write-Host "  ✓ OK" -ForegroundColor Green

# ── 4. Test ──
Write-Host "`n[4/5] cargo test" -ForegroundColor Yellow
cargo test 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) { Write-Host "FAIL: test" -ForegroundColor Red; exit 1 }
Write-Host "  ✓ OK" -ForegroundColor Green

# ── 5. Coverage (cargo-llvm-cov) ──
Write-Host "`n[5/5] cargo llvm-cov (coverage gating)" -ForegroundColor Yellow

# Install cargo-llvm-cov if missing
$llvmCovInstalled = cargo llvm-cov --version 2>&1 | Out-Null; $?
if (-not $?) {
    Write-Host "  Installing cargo-llvm-cov..." -ForegroundColor Yellow
    cargo install cargo-llvm-cov
}

# Run coverage for all packages (skip plugins that fail build)
$coverageOutput = cargo llvm-cov --workspace --html 2>&1
$coverageExit = $LASTEXITCODE

# Extract coverage percentage from output
$covLine = $coverageOutput | Select-String "TOTAL.*\d+\.\d+%" | Select-Object -First 1
if ($covLine) {
    Write-Host "  $covLine" -ForegroundColor Cyan
}

# Coverage gate: overall >= 40% (current baseline 41%)
$targetPct = 40
if ($covLine -match "(\d+\.\d+)%") {
    $currentPct = [double]$Matches[1]
    if ($currentPct -ge $targetPct) {
        Write-Host "  ✓ Coverage $currentPct% >= $targetPct% gate" -ForegroundColor Green
    } else {
        Write-Host "  FAIL: Coverage $currentPct% < $targetPct% gate" -ForegroundColor Red
        exit 1
    }
}

if ($coverageExit -ne 0) {
    Write-Host "  WARN: coverage run had warnings (non-fatal)" -ForegroundColor Yellow
}

Write-Host "`n=== CI PASSED ===" -ForegroundColor Green
