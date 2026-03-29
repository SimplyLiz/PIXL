-- commands/expand.lua — Convert a .paxl file to .pax TOML format
local cli = dofile(plugin.path .. "/lib/cli.lua")
local img = dofile(plugin.path .. "/lib/image.lua")

return function()
  local dlg = Dialog("Expand PAX-L")
  dlg:file{
    id = "paxl_file",
    label = "PAX-L file",
    filetypes = { "paxl" },
    open = true,
  }
  dlg:check{
    id = "strict",
    label = "Strict parsing (reject structural errors)",
    selected = false,
  }
  dlg:separator{ text = "Output" }
  dlg:file{
    id = "out_file",
    label = "Save as .pax",
    filetypes = { "pax" },
    save = true,
  }
  dlg:label{ text = "Leave empty to show in console" }
  dlg:separator()
  dlg:button{ id = "expand", text = "Expand" }
  dlg:button{ id = "cancel", text = "Cancel" }
  dlg:show()

  if not dlg.data.expand then return end

  local paxl_path = dlg.data.paxl_file
  if not paxl_path or paxl_path == "" then
    app.alert("Please select a .paxl file.")
    return
  end

  local args = { "expand" }
  if dlg.data.strict then args[#args + 1] = "--strict" end

  local ok, output, code = cli.exec_stdin(args, paxl_path)
  if not ok then
    app.alert("pixl expand failed:\n" .. (output or "exit code " .. code))
    return
  end

  local out_path = dlg.data.out_file
  if out_path and out_path ~= "" then
    -- Ensure .pax extension
    if not out_path:match("%.pax$") then
      out_path = out_path .. ".pax"
    end
    local f = io.open(out_path, "w")
    if f then
      f:write(output)
      f:close()
      app.alert("Saved PAX to " .. app.fs.fileName(out_path))
    else
      app.alert("Could not write to " .. out_path)
    end
  else
    local preview = output
    if #preview > 2000 then
      preview = preview:sub(1, 2000) .. "\n\n... (truncated)"
    end
    local dlg2 = Dialog("PAX Output")
    dlg2:label{ text = app.fs.fileName(paxl_path) .. " → PAX TOML" }
    dlg2:separator()
    dlg2:label{ text = #output .. " bytes" }
    dlg2:separator()
    dlg2:button{ id = "ok", text = "OK" }
    dlg2:show()
  end
end
