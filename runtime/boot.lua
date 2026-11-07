print("boot ok", orpheus.version, orpheus.paths.music, orpheus.paths.config)
local n = 0
function orpheus._event(name, ...)
    n = n + 1
    if n % 60 == 0 then print(name, ...) end
end
