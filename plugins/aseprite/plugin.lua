-- PIXL Aseprite Plugin
-- Bridge between Aseprite and the PIXL pixel-art engine.
--
-- Commands are registered under Sprite > PIXL in the menu bar.
-- Requires the `pixl` CLI binary — configure the path in PIXL > Settings.

function init(plugin)
  -- Preserve preferences across sessions
  if plugin.preferences == nil then
    plugin.preferences = {}
  end

  local group = "sprite_pixl"

  -- Import PAX tiles into Aseprite
  plugin:newCommand{
    id = "PixlImport",
    title = "Import PAX...",
    group = group,
    onenabled = function() return true end,
    onclick = function()
      local cmd = dofile(plugin.path .. "/commands/import_pax.lua")
      cmd()
    end,
  }

  -- Export current sprite to PAX tile
  plugin:newCommand{
    id = "PixlExport",
    title = "Export to PAX...",
    group = group,
    onenabled = function() return app.sprite ~= nil end,
    onclick = function()
      local cmd = dofile(plugin.path .. "/commands/export_pax.lua")
      cmd()
    end,
  }

  -- Render a PAX tile preview
  plugin:newCommand{
    id = "PixlRender",
    title = "Render Tile...",
    group = group,
    onenabled = function() return true end,
    onclick = function()
      local cmd = dofile(plugin.path .. "/commands/render.lua")
      cmd()
    end,
  }

  -- Critique a tile's pixel art quality
  plugin:newCommand{
    id = "PixlCritique",
    title = "Critique Tile...",
    group = group,
    onenabled = function() return true end,
    onclick = function()
      local cmd = dofile(plugin.path .. "/commands/critique.lua")
      cmd()
    end,
  }

  -- Pack atlas sprite sheet
  plugin:newCommand{
    id = "PixlAtlas",
    title = "Pack Atlas...",
    group = group,
    onenabled = function() return true end,
    onclick = function()
      local cmd = dofile(plugin.path .. "/commands/atlas.lua")
      cmd()
    end,
  }

  -- Convert image to pixel art
  plugin:newCommand{
    id = "PixlConvert",
    title = "Convert to Pixel Art...",
    group = group,
    onenabled = function() return true end,
    onclick = function()
      local cmd = dofile(plugin.path .. "/commands/convert.lua")
      cmd()
    end,
  }

  -- Validate PAX file
  plugin:newCommand{
    id = "PixlValidate",
    title = "Validate PAX...",
    group = group,
    onenabled = function() return true end,
    onclick = function()
      local cmd = dofile(plugin.path .. "/commands/validate.lua")
      cmd()
    end,
  }

  -- Settings
  plugin:newCommand{
    id = "PixlSettings",
    title = "PIXL Settings...",
    group = group,
    onenabled = function() return true end,
    onclick = function()
      local cmd = dofile(plugin.path .. "/commands/settings.lua")
      cmd()
    end,
  }
end

function exit(plugin)
  -- Nothing to clean up
end
