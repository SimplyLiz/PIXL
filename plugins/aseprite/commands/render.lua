-- commands/render.lua — Render a PAX tile as a preview in Aseprite
local cli = dofile(plugin.path .. "/lib/cli.lua")
local scan = dofile(plugin.path .. "/lib/pax_scan.lua")
local img = dofile(plugin.path .. "/lib/image.lua")

return function()
  local dlg = Dialog("PIXL Render")
  dlg:file{
    id = "pax_file",
    label = "PAX file",
    filetypes = { "pax" },
    open = true,
  }
  dlg:number{
    id = "scale",
    label = "Scale",
    text = "4",
    decimals = 0,
  }
  dlg:check{
    id = "grid",
    label = "Show pixel grid",
    selected = false,
  }
  dlg:button{ id = "scan", text = "Scan" }
  dlg:button{ id = "cancel", text = "Cancel" }
  dlg:show()

  if not dlg.data.scan then return end

  local pax_path = dlg.data.pax_file
  if not pax_path or pax_path == "" then return end

  local info = scan.scan(pax_path)
  if not info or #info.tiles == 0 then
    app.alert("No tiles found.")
    return
  end

  local scale = math.max(1, dlg.data.scale or 4)
  local use_grid = dlg.data.grid

  -- Select tile
  local dlg2 = Dialog("Select Tile")
  dlg2:combobox{
    id = "tile",
    label = "Tile",
    options = info.tiles,
  }
  dlg2:button{ id = "render", text = "Render" }
  dlg2:button{ id = "cancel", text = "Cancel" }
  dlg2:show()

  if not dlg2.data.render then return end

  local tile_name = dlg2.data.tile
  if not tile_name then return end

  local out_path = img.tmp(".png")

  -- Use preview command for grid overlay, render for plain output
  local args
  if use_grid then
    args = { "preview", pax_path, "--tile", tile_name, "-o", out_path, "--grid" }
  else
    args = { "render", pax_path, "--tile", tile_name, "--scale", tostring(scale), "-o", out_path }
  end

  local ok, output = cli.exec(args)
  if ok and app.fs.isFile(out_path) then
    app.open(out_path)
  else
    app.alert("Render failed:\n" .. (output or ""))
  end

  os.remove(out_path)
end
