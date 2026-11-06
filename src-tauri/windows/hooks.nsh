; Tauri's uninstaller deletes the sign-in (Run key) entry, but the agent's own
; autostart_initialized marker survives in its data folder, so a reinstall
; would never turn start-at-sign-in back on. Clear the marker too, so a
; reinstall starts like a first install. Updates (/UPDATE) keep both, the same
; as the Run key.
!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $UpdateMode <> 1
    SetShellVarContext current
    Delete "$LOCALAPPDATA\${BUNDLEID}\autostart_initialized"
  ${EndIf}
!macroend
