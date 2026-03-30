-- commands/import_pax.lua — Import tiles from a .pax file into Aseprite
local cli = dofile(plugin.path .. "/lib/cli.lua")
local scan = dofile(plugin.path .. "/lib/pax_scan.lua")
local img = dofile(plugin.path .. "/lib/image.lua")

return function()
  -- Step 1: Pick a .pax or .paxl file
  local dlg = Dialog("Import PAX")
  dlg:file{
    id = "pax_file",
    label = "PAX / PAX-L file",
    filetypes = { "pax", "paxl" },
    open = true,
  }
  dlg:number{
    id = "scale",
    label = "Preview scale",
    text = "1",
    decimals = 0,
  }
  dlg:check{
    id = "as_sheet",
    label = "Import as sprite sheet",
    selected = false,
  }
  dlg:button{ id = "next", text = "Scan" }
  dlg:button{ id = "cancel", text = "Cancel" }
  dlg:show()

  if not dlg.data.next then return end

  local pax_path = dlg.data.pax_file
  local scale = math.max(1, dlg.data.scale or 1)

  if not pax_path or pax_path == "" then
    app.alert("Please select a .pax or .paxl file.")
    return
  end

  -- Expand .paxl to temp .pax if needed
  local cli_pax, cleanup = cli.ensure_pax(pax_path)
  if not cli_pax then
    app.alert(cleanup)
    return
  end

  -- Step 2: Scan the file for tiles
  local info, err = scan.scan_auto(pax_path)
  if not info then
    app.alert("Scan failed: " .. (err or "unknown error"))
    return
  end

  if #info.tiles == 0 then
    app.alert("No tiles found in " .. app.fs.fileTitle(pax_path))
    return
  end

  -- Step 3: Let user pick tiles to import
  local dlg2 = Dialog("Select Tiles")
  dlg2:label{ text = #info.tiles .. " tiles found" }
  dlg2:separator()

  for _, name in ipairs(info.tiles) do
    dlg2:check{
      id = "tile_" .. name,
      label = name,
      selected = true,
    }
  end

  dlg2:separator()
  dlg2:button{ id = "import", text = "Import" }
  dlg2:button{ id = "all", text = "Select All" }
  dlg2:button{ id = "cancel", text = "Cancel" }
  dlg2:show()

  if not dlg2.data.import and not dlg2.data.all then return end

  -- Collect selected tiles
  local selected = {}
  for _, name in ipairs(info.tiles) do
    if dlg2.data.all or dlg2.data["tile_" .. name] then
      selected[#selected + 1] = name
    end
  end

  if #selected == 0 then
    app.alert("No tiles selected.")
    return
  end

  -- Step 4a: Atlas mode — import as a single sprite sheet
  if dlg.data.as_sheet then
    local atlas_png = img.tmp(".png")
    local map_json = img.tmp(".json")

    local ok, output = cli.exec({
      "atlas", cli_pax,
      "-o", atlas_png,
      "--map", map_json,
      "--scale", tostring(scale),
    })

    if ok and app.fs.isFile(atlas_png) then
      app.open(atlas_png)
      app.alert("Imported atlas from " .. app.fs.fileName(pax_path))
    else
      app.alert("Atlas generation failed:\n" .. (output or "unknown error"))
    end

    os.remove(atlas_png)
    os.remove(map_json)
    cleanup()
    return
  end

  -- Step 4b: Individual tile mode — render each tile as a separate sprite
  local imported = 0
  for _, name in ipairs(selected) do
    local out_path = img.tmp(".png")

    local ok, output = cli.exec({
      "render", cli_pax,
      "--tile", name,
      "--scale", tostring(scale),
      "-o", out_path,
    })

    if ok and app.fs.isFile(out_path) then
      local spr = app.open(out_path)
      if spr then
        -- Set sprite filename to tile name for reference
        spr.filename = name .. ".png"
        imported = imported + 1
      end
    end

    os.remove(out_path)
  end

  cleanup()

  app.alert("Imported " .. imported .. " of " .. #selected .. " tiles.")
end
