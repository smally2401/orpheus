local O = orpheus
O.window.set_title("hello from lua")
print("md5:", O.checksum("abc"))
local start = O.time()
function O._event(name, ...)
    if name == "frame" and O.time() - start > 3 then
        O.quit(7)
    end
end
