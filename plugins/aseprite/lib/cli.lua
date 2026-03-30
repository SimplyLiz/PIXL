-- lib/cli.lua — Execute pixl CLI commands and capture output
local M = {}

-- Resolve the configured pixl binary path, falling back to PATH lookup.
function M.binary()
  local pref = plugin.preferences or {}
  return pref.pixl_path or "pixl"
end

-- Shell-quote a single argument.
local function quote(s)
  return "'" .. s:gsub("'", "'\\''") .. "'"
end

-- Build a full command string from a list of arguments.
function M.build_cmd(args)
  local parts = { quote(M.binary()) }
  for _, a in ipairs(args) do
    parts[#parts + 1] = quote(tostring(a))
  end
  return table.concat(parts, " ")
end

-- Monotonic counter to avoid temp file collisions within the same tick.
local seq = 0

-- Execute a pixl command, redirect stdout+stderr to a temp file, return
-- (ok: bool, output: string, exit_code: number).
function M.exec(args)
  seq = seq + 1
  local tmp = app.fs.joinPath(app.fs.tempPath, "pixl_out_" .. os.clock() .. "_" .. seq .. ".txt")
  local cmd = M.build_cmd(args) .. " > " .. quote(tmp) .. " 2>&1"

  local ok, _, code = os.execute(cmd)
  -- os.execute returns (bool, "exit", code) in Lua 5.3
  code = code or 0

  local output = ""
  local f = io.open(tmp, "r")
  if f then
    output = f:read("*a") or ""
    f:close()
    os.remove(tmp)
  end

  return ok and (code == 0), output, code
end

-- Execute and return output on success, or nil + error message on failure.
function M.run(args)
  local ok, output, code = M.exec(args)
  if ok then
    return output
  end
  return nil, output, code
end

-- Execute a pixl command with stdin piped from a file.
-- Returns (ok: bool, output: string, exit_code: number).
function M.exec_stdin(args, stdin_path)
  seq = seq + 1
  local tmp = app.fs.joinPath(app.fs.tempPath, "pixl_out_" .. os.clock() .. "_" .. seq .. ".txt")
  local cmd = M.build_cmd(args) .. " < " .. quote(stdin_path) .. " > " .. quote(tmp) .. " 2>&1"

  local ok, _, code = os.execute(cmd)
  code = code or 0

  local output = ""
  local f = io.open(tmp, "r")
  if f then
    output = f:read("*a") or ""
    f:close()
    os.remove(tmp)
  end

  return ok and (code == 0), output, code
end

-- If path is a .paxl file, expand it to a temporary .pax file.
-- Returns (pax_path, cleanup_fn).  For .pax inputs, cleanup is a no-op.
function M.ensure_pax(path)
  if not path:match("%.paxl$") then
    return path, function() end
  end
  seq = seq + 1
  local tmp_pax = app.fs.joinPath(app.fs.tempPath, "pixl_expanded_" .. os.clock() .. "_" .. seq .. ".pax")
  local ok, output, code = M.exec_stdin({ "expand" }, path)
  if not ok then
    return nil, "pixl expand failed:\n" .. (output or "exit code " .. code)
  end
  local f = io.open(tmp_pax, "w")
  if not f then
    return nil, "cannot write temp file"
  end
  f:write(output)
  f:close()
  return tmp_pax, function() os.remove(tmp_pax) end
end

return M
