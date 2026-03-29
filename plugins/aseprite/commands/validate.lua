-- commands/validate.lua — Validate a PAX file and show the report
local cli = dofile(plugin.path .. "/lib/cli.lua")

return function()
  local dlg = Dialog("PIXL Validate")
  dlg:file{
    id = "pax_file",
    label = "PAX file",
    filetypes = { "pax" },
    open = true,
  }
  dlg:check{ id = "edges", label = "Check edge compatibility", selected = true }
  dlg:check{ id = "quality", label = "Quality analysis", selected = false }
  dlg:check{ id = "completeness", label = "Completeness check", selected = false }
  dlg:separator()
  dlg:button{ id = "run", text = "Validate" }
  dlg:button{ id = "cancel", text = "Cancel" }
  dlg:show()

  if not dlg.data.run then return end

  local pax_path = dlg.data.pax_file
  if not pax_path or pax_path == "" then return end

  local args = { "validate", pax_path }
  if dlg.data.edges then args[#args + 1] = "--check_edges" end
  if dlg.data.quality then args[#args + 1] = "--quality" end
  if dlg.data.completeness then args[#args + 1] = "--completeness" end

  local ok, output, code = cli.exec(args)
  local clean = output:gsub("\27%[[%d;]*m", "")

  local title = ok and "Validation Passed" or "Validation Issues Found"
  local dlg2 = Dialog(title)
  dlg2:label{ text = clean }
  dlg2:button{ id = "ok", text = "OK" }
  dlg2:show()
end
