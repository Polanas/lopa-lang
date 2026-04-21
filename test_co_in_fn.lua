local anchor = {}

anchor.co = coroutine.create(function()
	coroutine.yield(5)
	coroutine.yield(5)
	coroutine.yield(5)
end)

local ffi = require("ffi")
ffi.cdef([[
  typedef struct lua_State lua_State;
]])

local fnPtr = ffi.new("FnPtr", {
	ptr = ffi.cast("lua_State*", anchor.co),
})

local success, result = coroutine.resume(fnPtr.ptr)
print(success, result)
