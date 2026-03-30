-- commands/settings.lua — Configure the PIXL plugin
local cli = dofile(plugin.path .. "/lib/cli.lua")

return function()
  local pref = plugin.preferences or {}

  local dlg = Dialog("PIXL Settings")

  dlg:file{
    id = "pixl_path",
    label = "pixl binary",
    filename = pref.pixl_path or "",
    filetypes = {},
    open = true,
  }
  dlg:newrow()
  dlg:label{ text = "Leave empty to use 'pixl' from PATH" }
  dlg:separator()

  -- Test connection
  dlg:button{
    id = "test",
    text = "Test Connection",
    onclick = function()
      local ok, output = cli.exec({ "--version" })
      if ok then
        app.alert{ title = "PIXL", text = "Connected: " .. output:gsub("%s+$", "") }
      else
        app.alert{ title = "PIXL Error", text = "Could not run pixl:\n" .. output }
      end
    end,
  }

  dlg:button{ id = "ok", text = "Save" }
  dlg:button{ id = "cancel", text = "Cancel" }
  dlg:show()

  if dlg.data.ok then
    local path = dlg.data.pixl_path
    if path and path ~= "" then
      pref.pixl_path = path
    else
      pref.pixl_path = nil
    end
    plugin.preferences = pref
  end
end
