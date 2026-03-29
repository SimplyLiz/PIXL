-- lib/pax_scan.lua — Lightweight PAX file scanner
-- Extracts tile names, palette names, and sprite names without a full TOML parse.
local M = {}

-- Scan a .pax file and return { tiles = {}, palettes = {}, sprites = {} }
function M.scan(path)
  local result = { tiles = {}, palettes = {}, sprites = {}, composites = {} }

  local f = io.open(path, "r")
  if not f then return nil, "cannot open " .. path end

  for line in f:lines() do
    local section, name = line:match("^%[(%w+)%.([%w_%-]+)%]")
    if section and name then
      if section == "tile" then
        result.tiles[#result.tiles + 1] = name
      elseif section == "palette" then
        result.palettes[#result.palettes + 1] = name
      elseif section == "spriteset" then
        result.sprites[#result.sprites + 1] = name
      elseif section == "composite" then
        result.composites[#result.composites + 1] = name
      end
    end
  end
  f:close()

  return result
end

-- Extract a palette from a .pax file: returns { [symbol] = "#rrggbbaa", ... }
function M.read_palette(path, palette_name)
  local f = io.open(path, "r")
  if not f then return nil end

  local in_palette = false
  local colors = {}

  for line in f:lines() do
    -- Check for section header
    local sec, name = line:match("^%[(%w+)%.([%w_%-]+)%]")
    if sec then
      in_palette = (sec == "palette" and name == palette_name)
    elseif in_palette then
      -- Match "symbol" = "#hexcolor"
      local sym, hex = line:match('^%s*"(.-)"%s*=%s*"(#%x+)"')
      if sym and hex then
        -- Normalize to 8-digit hex
        if #hex == 7 then hex = hex .. "ff" end
        colors[sym] = hex
      end
    end
  end
  f:close()

  return colors
end

-- Read tile metadata: { palette = "name", size = "WxH", grid = "..." }
function M.read_tile(path, tile_name)
  local f = io.open(path, "r")
  if not f then return nil end

  local in_tile = false
  local tile = {}
  local grid_lines = {}
  local in_grid = false

  for line in f:lines() do
    local sec, name = line:match("^%[(%w+)%.([%w_%-]+)%]")
    if sec then
      if in_tile then break end -- we passed our tile
      in_tile = (sec == "tile" and name == tile_name)
    elseif in_tile then
      if in_grid then
        if line:match("^'''") then
          in_grid = false
          tile.grid = table.concat(grid_lines, "\n")
        else
          grid_lines[#grid_lines + 1] = line
        end
      else
        local key, val = line:match("^(%w+)%s*=%s*(.+)")
        if key then
          val = val:gsub('^"', ""):gsub('"$', "")
          if val == "'''" then
            in_grid = true
          else
            tile[key] = val
          end
        end
      end
    end
  end
  f:close()

  if not tile.palette then return nil end
  return tile
end

-- Scan a .paxl file and return { tiles = {}, palettes = {}, sprites = {}, composites = {} }
-- PAX-L uses line-oriented sigil directives: @pal, @tile, @sprite, @composite, @stamp
function M.scan_paxl(path)
  local result = { tiles = {}, palettes = {}, sprites = {}, composites = {}, stamps = {} }

  local f = io.open(path, "r")
  if not f then return nil, "cannot open " .. path end

  for line in f:lines() do
    local directive, name = line:match("^@(%w+)%s+([%w_%-]+)")
    if directive and name then
      if directive == "tile" then
        result.tiles[#result.tiles + 1] = name
      elseif directive == "pal" then
        result.palettes[#result.palettes + 1] = name
      elseif directive == "sprite" then
        result.sprites[#result.sprites + 1] = name
      elseif directive == "composite" then
        result.composites[#result.composites + 1] = name
      elseif directive == "stamp" then
        result.stamps[#result.stamps + 1] = name
      end
    end
  end
  f:close()

  return result
end

-- Detect format from file extension and scan accordingly.
function M.scan_auto(path)
  if path:match("%.paxl$") then
    return M.scan_paxl(path)
  else
    return M.scan(path)
  end
end

return M
