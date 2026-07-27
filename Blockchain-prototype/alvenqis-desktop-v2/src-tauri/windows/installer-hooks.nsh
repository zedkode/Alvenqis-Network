!macro ALVENQIS_STOP_DESKTOP_PROCESSES
  nsExec::ExecToLog 'taskkill /F /IM alvenqis-desktop-v2.exe'
  nsExec::ExecToLog 'taskkill /F /IM alvenqis-miner.exe'
  nsExec::ExecToLog 'taskkill /F /IM alvenqis-node.exe'
  nsExec::ExecToLog 'taskkill /F /IM alvenqis-rpc-gateway.exe'
  nsExec::ExecToLog 'taskkill /F /IM alvenqis-indexer.exe'
  Sleep 750
!macroend

!macro ALVENQIS_REMOVE_DESKTOP_DATA
  RMDir /r "$APPDATA\Alvenqis\ControlCenter"
  RMDir /r "$LOCALAPPDATA\Alvenqis\ControlCenter"
  RMDir /r "$APPDATA\Alvenqis\Desktop"
  RMDir /r "$LOCALAPPDATA\Alvenqis\Desktop"
  RMDir /r "$APPDATA\network.alvenqis.control-center-v2"
  RMDir /r "$LOCALAPPDATA\network.alvenqis.control-center-v2"
  RMDir /r "$APPDATA\Alvenqis Control Center V2"
  RMDir /r "$LOCALAPPDATA\Alvenqis Control Center V2"
!macroend

!macro NSIS_HOOK_PREINSTALL
  !insertmacro ALVENQIS_STOP_DESKTOP_PROCESSES
  Delete "$INSTDIR\alvenqis-desktop-v2.exe"
  Delete "$INSTDIR\alvenqis-keystore-helper.exe"
  Delete "$INSTDIR\resources\bin\alvenqis-miner.exe"
  Delete "$INSTDIR\resources\bin\alvenqis-node.exe"
  Delete "$INSTDIR\resources\bin\alvenqis-rpc-gateway.exe"
  Delete "$INSTDIR\resources\bin\alvenqis-indexer.exe"
!macroend

!macro NSIS_HOOK_POSTINSTALL
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro ALVENQIS_STOP_DESKTOP_PROCESSES
  ${If} ${FileExists} "$INSTDIR\alvenqis-keystore-helper.exe"
    nsExec::ExecToLog '"$INSTDIR\alvenqis-keystore-helper.exe" --purge-uninstall'
  ${ElseIf} ${FileExists} "$INSTDIR\resources\bin\alvenqis-keystore-helper.exe"
    nsExec::ExecToLog '"$INSTDIR\resources\bin\alvenqis-keystore-helper.exe" --purge-uninstall'
  ${EndIf}
  !insertmacro ALVENQIS_REMOVE_DESKTOP_DATA
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  !insertmacro ALVENQIS_REMOVE_DESKTOP_DATA
!macroend
