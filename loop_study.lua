--an iterator consists of 3 parts:
--1. iterator function
--2. invariant state
--3. initial value
--
--for var_1, ..., var_n in explist do block end
--is equal to
--do
--    local _f, _s, _var = explist
--    while true do
--      local var_1, ... , var_n = _f(_s, _var)
--      _var = var_1
--      if _var == nil then break end
--      block
--    end
-- end

function ipairs_iter(state, var)
  var = var + 1
	if var > #state then
		return nil, nil, nil
	else
    return var, state[var]
	end
end

function my_ipairs(t)
	return ipairs_iter, t, 0
end

for k, v in my_ipairs({ -1, -2, -3 }) do
	print(k,v)
end
print("finished")
