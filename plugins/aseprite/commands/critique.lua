-- commands/critique.lua — Run PIXL critique on a tile and display results
local cli = dofile(plugin.path .. "/lib/cli.lua")
local scan = dofile(plugin.path .. "/lib/pax_scan.lua")

return function()
  -- Step 1: Pick PAX file and tile
  local dlg = Dialog("PIXL Critique")
  dlg:file{
    id = "pax_file",
    label = "PAX / PAX-L file",
    filetypes = { "pax", "paxl" },
    open = true,
  }
  dlg:button{ id = "scan", text = "Scan Tiles" }
  dlg:button{ id = "cancel", text = "Cancel" }
  dlg:show()

  if not dlg.data.scan then return end

  local pax_path = dlg.data.pax_file
  if not pax_path or pax_path == "" then
    app.alert("Select a PAX file first.")
    return
  end

  local info = scan.scan_auto(pax_path)
  if not info or #info.tiles == 0 then
    app.alert("No tiles found.")
    return
  end

  -- Step 2: Select tile
  local dlg2 = Dialog("Select Tile to Critique")
  dlg2:combobox{
    id = "tile",
    label = "Tile",
    options = info.tiles,
  }
  dlg2:button{ id = "run", text = "Critique" }
  dlg2:button{ id = "cancel", text = "Cancel" }
  dlg2:show()

  if not dlg2.data.run then return end

  local tile_name = dlg2.data.tile
  if not tile_name then return end

  -- Step 3: Expand .paxl to temp .pax if needed
  local cli_pax, cleanup = cli.ensure_pax(pax_path)
  if not cli_pax then
    app.alert(cleanup)
    return
  end

  local ok, output, code = cli.exec({ "critique", cli_pax, "--tile", tile_name })

  -- Step 4: Display results
  -- Strip ANSI color codes for display
  local clean = output:gsub("\27%[[%d;]*m", "")

  local dlg3 = Dialog("Critique: " .. tile_name)
  dlg3:label{ text = "pixl critique " .. tile_name }
  dlg3:separator()
  dlg3:label{ text = clean }
  dlg3:separator()

  -- Show verdict prominently
  local verdict = clean:match("[Vv]erdict:%s*(%w+)")
  if verdict then
    dlg3:label{ text = "Verdict: " .. verdict }
  end

  dlg3:button{ id = "ok", text = "OK" }
  dlg3:show()

  cleanup()
end
