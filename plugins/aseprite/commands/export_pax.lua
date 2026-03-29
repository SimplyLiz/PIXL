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
  dlg:separator{ text = "Target file" }
  dlg:file{
    id = "pax_file",
    label = "Append to .pax / .paxl",
    filetypes = { "pax", "paxl" },
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

  -- Step 3: If we have a target file, use `pixl import` to quantize against its palette
  local is_paxl = pax_path and pax_path:match("%.paxl$")

  if pax_path and pax_path ~= "" then
    -- Expand .paxl to temp .pax so `pixl import` can read the palette
    local import_pax, expand_cleanup = cli.ensure_pax(pax_path)
    if not import_pax then
      os.remove(tmp_png)
      app.alert(expand_cleanup)
      return
    end

    local args = {
      "import", tmp_png,
      "--size", size_str,
      "--pax", import_pax,
      "--palette", palette_name,
    }
    if dlg.data.dither then
      args[#args + 1] = "--dither"
    end

    local output, err_msg = cli.run(args)
    os.remove(tmp_png)
    expand_cleanup()

    if output then
      -- Strip leading comment lines (# ...) and trailing whitespace
      local grid = output:gsub("^(#[^\n]*\n)+", ""):gsub("%s+$", "")

      if is_paxl then
        -- Build PAX-L tile block
        local paxl_block = string.format(
          "\n@tile %s %s pal=%s\n%s\n",
          tile_name, size_str, palette_name, grid
        )
        local f = io.open(pax_path, "a")
        if f then
          f:write(paxl_block)
          f:close()
          app.alert("Tile '" .. tile_name .. "' appended to " .. app.fs.fileName(pax_path))
        else
          app.alert("Could not write to " .. pax_path .. "\n\nGenerated block:\n" .. paxl_block)
        end
      else
        -- Build TOML tile block
        local tile_block = string.format(
          '\n[tile.%s]\npalette = "%s"\nsize = "%s"\ngrid = \'\'\'\n%s\n\'\'\'\n',
          tile_name, palette_name, size_str, grid
        )
        local f = io.open(pax_path, "a")
        if f then
          f:write(tile_block)
          f:close()
          app.alert("Tile '" .. tile_name .. "' appended to " .. app.fs.fileName(pax_path))
        else
          app.alert("Could not write to " .. pax_path .. "\n\nGenerated block:\n" .. tile_block)
        end
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
