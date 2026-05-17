function wrap(f)
  f()
end

local t = {}
UPVALUES = {}
local upvalues = UPVALUES

function lambda()
  local t = UPVALUES[1]
  local i = UPVALUES[2]
  t[i] = i
end

upvalues[1] = t
for i = 1, 1e6 do
  upvalues[2] = i
  wrap(lambda)
end
