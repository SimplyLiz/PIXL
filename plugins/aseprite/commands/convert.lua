-- commands/convert.lua — Convert an image to pixel art using PIXL's engine
local cli = dofile(plugin.path .. "/lib/cli.lua")
local img = dofile(plugin.path .. "/lib/image.lua")

return function()
  local dlg = Dialog("PIXL Convert to Pixel Art")

  dlg:file{
    id = "input",
    label = "Source image",
    filetypes = { "png", "jpg", "jpeg", "bmp", "gif", "webp" },
    open = true,
  }
  dlg:number{
    id = "width",
    label = "Target width (px)",
    text = "32",
    decimals = 0,
  }
  dlg:number{
    id = "colors",
    label = "Max colors",
    text = "16",
    decimals = 0,
  }
  dlg:number{
    id = "preview_scale",
    label = "Preview scale",
    text = "4",
    decimals = 0,
  }
  dlg:check{
    id = "use_active",
    label = "Use active sprite instead of file",
    selected = false,
  }
  dlg:separator()
  dlg:button{ id = "convert", text = "Convert" }
  dlg:button{ id = "cancel", text = "Cancel" }
  dlg:show()

  if not dlg.data.convert then return end

  -- Determine input source
  local input_path
  if dlg.data.use_active then
    input_path = img.save_tmp_png()
    if not input_path then
      app.alert("No active sprite to convert.")
      return
    end
  else
    input_path = dlg.data.input
    if not input_path or input_path == "" then
      app.alert("Select an image file.")
      return
    end
  end

  local out_dir = app.fs.joinPath(app.fs.tempPath, "pixl_convert_" .. os.clock())
  local width = math.max(8, dlg.data.width or 32)
  local colors = math.max(2, dlg.data.colors or 16)
  local preview = math.max(1, dlg.data.preview_scale or 4)

  local ok, output = cli.exec({
    "convert", input_path,
    "--width", tostring(width),
    "--colors", tostring(colors),
    "--preview", tostring(preview),
    "-o", out_dir,
  })

  if ok then
    -- Open the first output PNG
    local files = app.fs.listFiles(out_dir)
    local opened = 0
    for _, fname in ipairs(files or {}) do
      if fname:match("%.png$") then
        local full = app.fs.joinPath(out_dir, fname)
        app.open(full)
        opened = opened + 1
      end
    end

    if opened > 0 then
      app.alert("Converted " .. opened .. " image(s) at " .. width .. "px, " .. colors .. " colors.")
    else
      app.alert("Conversion produced no output files.\n" .. (output or ""))
    end
  else
    app.alert("Convert failed:\n" .. (output or ""))
  end

  -- Cleanup temp input if we saved one
  if dlg.data.use_active and input_path then
    os.remove(input_path)
  end
end
