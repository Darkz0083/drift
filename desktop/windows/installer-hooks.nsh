; Drift registers itself as a browser per user when asked to become the
; default (desktop/src/default_browser/windows.rs). Uninstalling removes that
; registration so Default apps never lists a browser that is gone.
!macro NSIS_HOOK_POSTUNINSTALL
  DeleteRegKey HKCU "Software\Clients\StartMenuInternet\Drift"
  DeleteRegKey HKCU "Software\Classes\DriftURL"
  DeleteRegValue HKCU "Software\RegisteredApplications" "Drift"
!macroend
