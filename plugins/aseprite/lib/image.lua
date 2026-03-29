-- lib/image.lua — Helpers for converting between Aseprite images and PIXL data
local M = {}

-- Create a temporary file path with the given extension.
function M.tmp(ext)
  return app.fs.joinPath(
    app.fs.tempPath,
    "pixl_" .. tostring(os.clock()):gsub("%.", "") .. (ext or ".png")
  )
end

-- Save the active sprite (or a given sprite) to a temporary PNG, return the path.
function M.save_tmp_png(sprite)
  sprite = sprite or app.sprite
  if not sprite then return nil, "no active sprite" end

  local path = M.tmp(".png")
  sprite:saveCopyAs(path)
  return path
end

-- Open a PNG file as a new Aseprite sprite and return it.
function M.open_png(path)
  if not app.fs.isFile(path) then
    return nil, "file not found: " .. path
  end
  return app.open(path)
end

-- Build an Aseprite Palette object from a PAX palette table { sym = "#rrggbbaa" }.
-- Returns (Palette, symbol_order) where symbol_order[i] = symbol for color index i.
function M.palette_from_pax(pax_colors)
  -- Sort symbols for deterministic ordering (transparent first if present)
  local syms = {}
  for sym in pairs(pax_colors) do
    syms[#syms + 1] = sym
  end
  table.sort(syms, function(a, b)
    -- Put transparent (".") first
    if a == "." then return true end
    if b == "." then return false end
    return a < b
  end)

  local pal = Palette(#syms)
  for i, sym in ipairs(syms) do
    local hex = pax_colors[sym]
    local r = tonumber(hex:sub(2, 3), 16)
    local g = tonumber(hex:sub(4, 5), 16)
    local b = tonumber(hex:sub(6, 7), 16)
    local a = tonumber(hex:sub(8, 9), 16) or 255
    pal:setColor(i - 1, Color(r, g, b, a))
  end

  return pal, syms
end

-- Map an Aseprite Image (indexed mode) back to a PAX grid string.
-- symbol_order: table where symbol_order[index+1] = pax symbol
function M.image_to_grid(image, symbol_order)
  local rows = {}
  for y = 0, image.height - 1 do
    local row = {}
    for x = 0, image.width - 1 do
      local idx = image:getPixel(x, y)
      row[#row + 1] = symbol_order[idx + 1] or "."
    end
    rows[#rows + 1] = table.concat(row)
  end
  return table.concat(rows, "\n")
end

return M
