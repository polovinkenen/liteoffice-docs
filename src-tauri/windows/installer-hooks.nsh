!macro LiteOfficeRegisterProgId PROGID DESCRIPTION
  WriteRegStr SHELL_CONTEXT "Software\Classes\${PROGID}" "" "${DESCRIPTION}"
  WriteRegStr SHELL_CONTEXT "Software\Classes\${PROGID}\DefaultIcon" "" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\",0"
  WriteRegStr SHELL_CONTEXT "Software\Classes\${PROGID}\shell" "" "open"
  WriteRegStr SHELL_CONTEXT "Software\Classes\${PROGID}\shell\open" "" "Open with LiteOffice Docs"
  WriteRegStr SHELL_CONTEXT "Software\Classes\${PROGID}\shell\open\command" "" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\" $\"%1$\""
!macroend

!macro LiteOfficeRegisterOpenWith EXT PROGID
  WriteRegStr SHELL_CONTEXT "Software\Classes\.${EXT}\OpenWithProgids" "${PROGID}" ""
!macroend

!macro LiteOfficeUnregisterOpenWith EXT PROGID
  DeleteRegValue SHELL_CONTEXT "Software\Classes\.${EXT}\OpenWithProgids" "${PROGID}"
  DeleteRegKey /ifempty SHELL_CONTEXT "Software\Classes\.${EXT}\OpenWithProgids"
  DeleteRegKey /ifempty SHELL_CONTEXT "Software\Classes\.${EXT}"
!macroend

!macro LiteOfficeUnregisterProgId PROGID
  DeleteRegKey SHELL_CONTEXT "Software\Classes\${PROGID}"
!macroend

!macro NSIS_HOOK_POSTINSTALL
  !insertmacro LiteOfficeRegisterProgId "LiteOfficeDocs.Word" "Word document"
  !insertmacro LiteOfficeRegisterProgId "LiteOfficeDocs.Rtf" "Rich Text Format document"
  !insertmacro LiteOfficeRegisterProgId "LiteOfficeDocs.Text" "Text document"
  !insertmacro LiteOfficeRegisterProgId "LiteOfficeDocs.Markdown" "Markdown document"
  !insertmacro LiteOfficeRegisterProgId "LiteOfficeDocs.Html" "HTML document"

  !insertmacro LiteOfficeRegisterOpenWith "docx" "LiteOfficeDocs.Word"
  !insertmacro LiteOfficeRegisterOpenWith "rtf" "LiteOfficeDocs.Rtf"
  !insertmacro LiteOfficeRegisterOpenWith "txt" "LiteOfficeDocs.Text"
  !insertmacro LiteOfficeRegisterOpenWith "md" "LiteOfficeDocs.Markdown"
  !insertmacro LiteOfficeRegisterOpenWith "markdown" "LiteOfficeDocs.Markdown"
  !insertmacro LiteOfficeRegisterOpenWith "html" "LiteOfficeDocs.Html"
  !insertmacro LiteOfficeRegisterOpenWith "htm" "LiteOfficeDocs.Html"
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  !insertmacro LiteOfficeUnregisterOpenWith "docx" "LiteOfficeDocs.Word"
  !insertmacro LiteOfficeUnregisterOpenWith "rtf" "LiteOfficeDocs.Rtf"
  !insertmacro LiteOfficeUnregisterOpenWith "txt" "LiteOfficeDocs.Text"
  !insertmacro LiteOfficeUnregisterOpenWith "md" "LiteOfficeDocs.Markdown"
  !insertmacro LiteOfficeUnregisterOpenWith "markdown" "LiteOfficeDocs.Markdown"
  !insertmacro LiteOfficeUnregisterOpenWith "html" "LiteOfficeDocs.Html"
  !insertmacro LiteOfficeUnregisterOpenWith "htm" "LiteOfficeDocs.Html"

  !insertmacro LiteOfficeUnregisterProgId "LiteOfficeDocs.Word"
  !insertmacro LiteOfficeUnregisterProgId "LiteOfficeDocs.Rtf"
  !insertmacro LiteOfficeUnregisterProgId "LiteOfficeDocs.Text"
  !insertmacro LiteOfficeUnregisterProgId "LiteOfficeDocs.Markdown"
  !insertmacro LiteOfficeUnregisterProgId "LiteOfficeDocs.Html"
!macroend
