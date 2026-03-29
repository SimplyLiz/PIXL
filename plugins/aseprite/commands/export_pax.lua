-- commands/export_pax.lua — Export current Aseprite sprite to PAX tile format
local cli = dofile(plugin.path .. "/lib/cli.lua")
local scan = dofile(plugin.path .. "/lib/pax_scan.lua")
local img = dofile(plugin.path .. "/lib/image.lua")

return function()
  local sprite = app.sprite
  if not sprite then
    app.alert("No active sprite.")
    return
  end

  -- Step 1: Configure export
  local dlg = Dialog("Export to PAX")

  dlg:entry{
    id = "tile_name",
    label = "Tile name",
    text = app.fs.fileTitle(sprite.filename):gsub("[^%w_]", "_"),
  }
  dlg:separator{ text = "Target PAX file" }
  dlg:file{
    id = "pax_file",
    label = "Append to .pax",
    filetypes = { "pax" },
    open = true,
  }
  dlg:label{ text = "Leave empty to print to console" }
  dlg:separator{ text = "Palette" }
  dlg:entry{
    id = "palette_name",
    label = "Palette name",
    text = "default",
  }
  dlg:check{
    id = "dither",
    label = "Enable dithering",
    selected = false,
  }
  dlg:separator()
  dlg:button{ id = "export", text = "Export" }
  dlg:button{ id = "cancel", text = "Cancel" }
  dlg:show()

  if not dlg.data.export then return end

  local tile_name = dlg.data.tile_name or "untitled"
  local pax_path = dlg.data.pax_file
  local palette_name = dlg.data.palette_name or "default"

  -- Step 2: Save current sprite to temp PNG
  local tmp_png, err = img.save_tmp_png(sprite)
  if not tmp_png then
    app.alert("Failed to save sprite: " .. (err or ""))
    return
  end

  local w = sprite.width
  local h = sprite.height
  local size_str = w .. "x" .. h

  -- Step 3: If we have a target PAX, use `pixl import` to quantize against its palette
  if pax_path and pax_path ~= "" then
    local args = {
      "import", tmp_png,
      "--size", size_str,
      "--pax", pax_path,
      "--palette", palette_name,
    }
    if dlg.data.dither then
      args[#args + 1] = "--dither"
    end

    local output, err_msg = cli.run(args)
    os.remove(tmp_png)

    if output then
      -- Build the TOML tile block
      -- Strip leading comment lines (# ...) and trailing whitespace
      local grid = output:gsub("^(#[^\n]*\n)+", ""):gsub("%s+$", "")
      local tile_block = string.format(
        '\n[tile.%s]\npalette = "%s"\nsize = "%s"\ngrid = \'\'\'\n%s\n\'\'\'\n',
        tile_name, palette_name, size_str, grid
      )

      -- Append to PAX file
      local f = io.open(pax_path, "a")
      if f then
        f:write(tile_block)
        f:close()
        app.alert("Tile '" .. tile_name .. "' appended to " .. app.fs.fileName(pax_path))
      else
        app.alert("Could not write to " .. pax_path .. "\n\nGenerated block:\n" .. tile_block)
      end
    else
      app.alert("pixl import failed:\n" .. (err_msg or "unknown error"))
    end
  else
    -- No target PAX: use `pixl convert` for standalone conversion
    local out_dir = img.tmp("")
    local ok, output = cli.exec({
      "convert", tmp_png,
      "--width", tostring(w),
      "--colors", "16",
      "-o", out_dir,
    })
    os.remove(tmp_png)

    if ok then
      app.alert("Converted. Output at:\n" .. out_dir)
    else
      app.alert("pixl convert failed:\n" .. (output or ""))
    end
  end
end
