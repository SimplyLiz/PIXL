-- commands/compact.lua — Convert a .pax file to PAX-L compact format
local cli = dofile(plugin.path .. "/lib/cli.lua")
local img = dofile(plugin.path .. "/lib/image.lua")

return function()
  local dlg = Dialog("Compact to PAX-L")
  dlg:file{
    id = "pax_file",
    label = "PAX file",
    filetypes = { "pax" },
    open = true,
  }
  dlg:check{
    id = "no_stamps",
    label = "Disable auto-stamp extraction",
    selected = false,
  }
  dlg:check{
    id = "no_row_refs",
    label = "Disable row references",
    selected = false,
  }
  dlg:check{
    id = "no_fill",
    label = "Disable fill detection",
    selected = false,
  }
  dlg:separator{ text = "Output" }
  dlg:file{
    id = "out_file",
    label = "Save as .paxl",
    filetypes = { "paxl" },
    save = true,
  }
  dlg:label{ text = "Leave empty to show in console" }
  dlg:separator()
  dlg:button{ id = "compact", text = "Compact" }
  dlg:button{ id = "cancel", text = "Cancel" }
  dlg:show()

  if not dlg.data.compact then return end

  local pax_path = dlg.data.pax_file
  if not pax_path or pax_path == "" then
    app.alert("Please select a .pax file.")
    return
  end

  local args = { "compact", pax_path }
  if dlg.data.no_stamps then args[#args + 1] = "--no-stamps" end
  if dlg.data.no_row_refs then args[#args + 1] = "--no-row-refs" end
  if dlg.data.no_fill then args[#args + 1] = "--no-fill" end

  local ok, output, code = cli.exec(args)
  if not ok then
    app.alert("pixl compact failed:\n" .. (output or "exit code " .. code))
    return
  end

  local out_path = dlg.data.out_file
  if out_path and out_path ~= "" then
    -- Ensure .paxl extension
    if not out_path:match("%.paxl$") then
      out_path = out_path .. ".paxl"
    end
    local f = io.open(out_path, "w")
    if f then
      f:write(output)
      f:close()
      app.alert("Saved PAX-L to " .. app.fs.fileName(out_path))
    else
      app.alert("Could not write to " .. out_path)
    end
  else
    -- Show output in a dialog
    local preview = output
    if #preview > 2000 then
      preview = preview:sub(1, 2000) .. "\n\n... (truncated)"
    end
    local dlg2 = Dialog("PAX-L Output")
    dlg2:label{ text = app.fs.fileName(pax_path) .. " → PAX-L" }
    dlg2:separator()
    dlg2:label{ text = #output .. " bytes" }
    dlg2:separator()
    dlg2:button{ id = "ok", text = "OK" }
    dlg2:show()
  end
end
