local O, g = orpheus, orpheus.gfx
function O._event(name, ...)
    if name ~= "frame" then
        print(name, ...)
        return
    end
    local dt = ...
    local w, h = O.window.size()
    g.rect(20, 20, 200, 100, "#6aa3ff")
    g.outline(20, 140, 200, 100, { 255, 100, 100 }, 3)
    g.poly({ 300, 20, 360, 60, 300, 100 }, "#e0a458")
    g.clip(400, 20, 100, 30)
    g.text("this text is clipped at 100 px wide", 400, 30, "#ffffff")
    g.unclip()
    g.text(string.format("dt = %.4f", dt), 20, h - 20, "#8b8d9c80")
end
