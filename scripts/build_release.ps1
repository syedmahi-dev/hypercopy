$releaseDir = "C:\Users\Mahi\.gemini\antigravity\scratch\hypercopy\release"
$stageDir = "C:\Users\Mahi\AppData\Local\Temp\hcp_stage_v2"
Remove-Item -Recurse -Force $releaseDir, $stageDir -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $releaseDir | Out-Null
New-Item -ItemType Directory -Force -Path $stageDir | Out-Null

Copy-Item "C:\Users\Mahi\.cargo\bin\hcp.exe" $stageDir
Copy-Item "C:\Users\Mahi\.cargo\bin\hypercopy.exe" $stageDir
Copy-Item "C:\Users\Mahi\.gemini\antigravity\scratch\hypercopy\README.md" $stageDir
Copy-Item "C:\Users\Mahi\.gemini\antigravity\scratch\hypercopy\BENCHMARKS.md" $stageDir
Copy-Item "C:\Users\Mahi\.gemini\antigravity\scratch\hypercopy\USAGE.md" $stageDir
Copy-Item "C:\Users\Mahi\.gemini\antigravity\scratch\hypercopy\CHANGELOG.md" $stageDir
Copy-Item "C:\Users\Mahi\.gemini\antigravity\scratch\hypercopy\LICENSE" $stageDir

$zipPath = "$releaseDir\hypercopy-v0.2.0-windows-x86_64.zip"
Compress-Archive -Path "$stageDir\*" -DestinationPath $zipPath -Force
Copy-Item "C:\Users\Mahi\.gemini\antigravity\scratch\hypercopy\npm\hypercopy-0.2.0.tgz" $releaseDir

$zipHash = (Get-FileHash -Algorithm SHA256 $zipPath).Hash.ToLower()
$tgzHash = (Get-FileHash -Algorithm SHA256 "$releaseDir\hypercopy-0.2.0.tgz").Hash.ToLower()
$sums = "$zipHash  hypercopy-v0.2.0-windows-x86_64.zip`n$tgzHash  hypercopy-0.2.0.tgz"
Set-Content -Path "$releaseDir\SHA256SUMS.txt" -Value $sums

Remove-Item -Recurse -Force $stageDir -ErrorAction SilentlyContinue

Write-Host "=== RELEASE v0.2.0 ARTIFACTS CREATED ==="
Get-ChildItem $releaseDir | Select-Object Name, Length
