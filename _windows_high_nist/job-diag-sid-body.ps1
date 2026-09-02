$py = 'C:\Program Files\Python312\python.exe'
& $py -c "import ctypes; a=ctypes.WinDLL('advapi32', use_last_error=True); print('StringSidToSidW', hasattr(a,'StringSidToSidW')); print('StringSidToSidA', hasattr(a,'StringSidToSidA')); print('StringSidToSid', hasattr(a,'StringSidToSid')); print('ConvertStringSidToSidW', hasattr(a,'ConvertStringSidToSidW'))" 2>&1 | ForEach-Object { Write-Output "PY: $_" }
$sig = Get-Command -Name 'StringSidToSidW' -ErrorAction SilentlyContinue
Write-Output "DLL_EXPORT_CHECK=$(if ($sig) { 'cmdlet' } else { 'n/a' })"
