-- Demonstrates on_song_change: a minimal example that appends every song
-- change to a text file on the desktop, with a timestamp. See the config
-- README's "Scripting hooks" section for the full reference, and
-- 06_lastfm.lua for a more involved example using this same hook plus
-- on_song_halfway.

local function get_desktop_path()
    local is_windows = package.config:sub(1,1) == "\\"

    if is_windows then
        local user_profile = os.getenv("USERPROFILE")
        if user_profile then
            return user_profile .. "\\Desktop"
        end

    else
        local home = os.getenv("HOME")
        if home then
            return home .. "/Desktop"
        end
    end

    return nil, "Could not determine home directory"
end

function on_song_change(song)
    local desktop_path = get_desktop_path()
    if desktop_path then
        local file_path = desktop_path .. "/song_history.txt"
        local file, err = io.open(file_path, "a")

        if file then
            local date = os.date("%d/%m/%Y")
            local time = os.date("%H:%M:%S")

            file:write("[" .. date .. " " .. time .. "] " .. song.artist .. " - " .. song.title .. "\n")
            file:close()

        else
            print("Failed to open song history:", err)
        end
    end
end
