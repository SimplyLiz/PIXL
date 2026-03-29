-- commands/atlas.lua — Pack a PAX tileset into a sprite sheet atlas
local cli = dofile(plugin.path .. "/lib/cli.lua")
local img = dofile(plugin.path .. "/lib/image.lua")

return function()
  local dlg = Dialog("PIXL Atlas Pack")
  dlg:file{
    id = "pax_file",
    label = "PAX file",
    filetypes = { "pax" },
    open = true,
  }
  dlg:number{
    id = "columns",
    label = "Columns",
    text = "8",
    decimals = 0,
  }
  dlg:number{
    id = "padding",
    label = "Padding (px)",
    text = "1",
    decimals = 0,
  }
  dlg:number{
    id = "scale",
    label = "Scale",
    text = "1",
    decimals = 0,
  }
  dlg:check{
    id = "save_map",
    label = "Generate JSON metadata",
    selected = true,
  }
  dlg:separator()
  dlg:button{ id = "pack", text = "Pack Atlas" }
  dlg:button{ id = "cancel", text = "Cancel" }
  dlg:show()

  if not dlg.data.pack then return end

  local pax_path = dlg.data.pax_file
  if not pax_path or pax_path == "" then return end

  local atlas_png = img.tmp(".png")
  local args = {
    "atlas", pax_path,
    "-o", atlas_png,
    "--columns", tostring(dlg.data.columns or 8),
    "--padding", tostring(dlg.data.padding or 1),
    "--scale", tostring(math.max(1, dlg.data.scale or 1)),
  }

  local map_json
  if dlg.data.save_map then
    map_json = img.tmp(".json")
    args[#args + 1] = "--map"
    args[#args + 1] = map_json
  end

  local ok, output = cli.exec(args)
  if ok and app.fs.isFile(atlas_png) then
    app.open(atlas_png)

    if map_json and app.fs.isFile(map_json) then
      -- Read JSON metadata to show summary
      local f = io.open(map_json, "r")
      if f then
        local json_text = f:read("*a")
        f:close()
        -- Count entries
        local count = 0
        for _ in json_text:gmatch('"filename"') do count = count + 1 end
        app.alert("Atlas packed: " .. count .. " tiles\nJSON metadata saved.")
      end
    end
  else
    app.alert("Atlas pack failed:\n" .. (output or ""))
  end

  -- Cleanup
  if atlas_png then os.remove(atlas_png) end
  if map_json then os.remove(map_json) end
end
