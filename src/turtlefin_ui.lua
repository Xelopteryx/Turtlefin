-- Turtlefin : interface de lecture dessinée DANS la vidéo par mpv (barre de contrôle + menus des pistes).
-- Rien de Slint ne peut s'afficher par-dessus la vidéo intégrée (fenêtre enfant) : l'interface vit donc ici.
-- Turtlefin écrit ce fichier dans le dossier temporaire et le charge avec --script : mpv le nomme « turtlefin_ui ».
--
-- Couleurs reprises de Style.css : verre sombre rgba(24, 24, 28), contour blanc translucide,
-- boutons « pilule », dégradé #a95bc2 -> #00a4db sur l'élément actif (pas de flou, pas d'ombre).
--
-- Touches : envoyées par Turtlefin (script-message-to turtlefin_ui key <nom>) quand sa fenêtre a le focus,
-- ou reçues directement par mpv après un clic sur la vidéo. Souris : survol + clic.

local mp = require 'mp'

local ACCENT1 = { 0xa9, 0x5b, 0xc2 }
local ACCENT2 = { 0x00, 0xa4, 0xdb }
local WHITE = "&HFFFFFF&"
local HIDE_AFTER = 3   -- secondes sans activité avant de masquer la barre
local ROWS = 8         -- lignes visibles dans un menu de pistes

-- ---------------------------------------------------------------------------
-- Dessin ASS
-- ---------------------------------------------------------------------------
local function bgr(r, g, b) return string.format("&H%02X%02X%02X&", b, g, r) end
local GLASS = bgr(24, 24, 28)

-- Opacité 0..1 -> alpha ASS (00 = opaque, FF = transparent).
local function alpha(a) return string.format("&H%02X&", math.floor((1 - a) * 255 + 0.5)) end

local function n(v) return math.floor(v + 0.5) end

local function esc(s)
    s = tostring(s or ""):gsub("[\\{}]", { ["\\"] = "/", ["{"] = "(", ["}"] = ")" }):gsub("\n", " ")
    return s
end

local function rrect(x, y, w, h, r)
    r = math.min(r, h / 2, w / 2)
    x, y, w, h, r = n(x), n(y), n(w), n(h), n(r)
    local x2, y2 = x + w, y + h
    return string.format(
        "m %d %d l %d %d b %d %d %d %d %d %d l %d %d b %d %d %d %d %d %d l %d %d b %d %d %d %d %d %d l %d %d b %d %d %d %d %d %d",
        x + r, y, x2 - r, y, x2, y, x2, y, x2, y + r, x2, y2 - r, x2, y2, x2, y2, x2 - r, y2,
        x + r, y2, x, y2, x, y2, x, y2 - r, x, y + r, x, y, x, y, x + r, y)
end

local W, H, S = 1280, 720, 1
local out, hit = {}, {}

local function shape(path, color, a, extra)
    out[#out + 1] = string.format("{\\an7\\pos(0,0)\\bord0\\shad0\\1c%s\\1a%s%s\\p1}%s",
        color, alpha(a), extra or "", path)
end

-- Contour fin blanc translucide (comme « border: 1px solid rgba(255,255,255,.14) »).
local function outline(path, a)
    out[#out + 1] = string.format("{\\an7\\pos(0,0)\\bord%d\\shad0\\1a&HFF&\\3c%s\\3a%s\\p1}%s",
        math.max(1, n(S)), WHITE, alpha(a), path)
end

-- ASS n'a pas de dégradé : bandes verticales découpées dans la forme.
local function gradient(x, y, w, h, r)
    local path = rrect(x, y, w, h, r)
    local steps = 16
    for i = 0, steps - 1 do
        local t = (i + 0.5) / steps
        local c = bgr(n(ACCENT1[1] + (ACCENT2[1] - ACCENT1[1]) * t),
            n(ACCENT1[2] + (ACCENT2[2] - ACCENT1[2]) * t),
            n(ACCENT1[3] + (ACCENT2[3] - ACCENT1[3]) * t))
        local x0, x1 = n(x + w * i / steps), n(x + w * (i + 1) / steps) + 1
        shape(path, c, 1, string.format("\\clip(%d,%d,%d,%d)", x0, n(y) - 1, x1, n(y + h) + 1))
    end
end

-- an : alignement ASS (4 = gauche-milieu, 5 = centre, 6 = droite-milieu).
local function text(x, y, s, size, an, o)
    o = o or {}
    local clip = o.clip and string.format("\\clip(%d,%d,%d,%d)", n(o.clip[1]), n(o.clip[2]), n(o.clip[3]), n(o.clip[4])) or ""
    out[#out + 1] = string.format("{\\an%d\\pos(%d,%d)\\fs%d\\b%d\\bord0\\shad0\\q2\\1c%s\\1a%s%s}%s",
        an, n(x), n(y), n(size), o.bold and 1 or 0, WHITE, alpha(o.a or 0.92), clip, esc(s))
end

local function glass(x, y, w, h)
    local p = rrect(x, y, w, h, 16 * S)
    shape(p, GLASS, 0.82)
    outline(p, 0.14)
end

local function zone(x, y, w, h, act, idx)
    hit[#hit + 1] = { x = x, y = y, w = w, h = h, act = act, idx = idx }
end

-- ---------------------------------------------------------------------------
-- État
-- ---------------------------------------------------------------------------
local BUTTONS = {
    { id = "pause", w = 58 },
    { id = "back",  label = "-10 s",       w = 84 },
    { id = "fwd",   label = "+10 s",       w = 84 },
    { id = "audio", label = "Audio",       w = 110, right = true },
    { id = "sub",   label = "Sous-titres", w = 150, right = true },
    { id = "stop",  label = "Arrêter",     w = 120, right = true },
}

local st = {
    visible = false,
    focus = nil,   -- index dans BUTTONS (navigation clavier / survol)
    panel = nil,   -- { kind, entries, sel, top }
    mx = -1, my = -1, hover = false, over_ui = false,
    paused = false, pos = 0, dur = 0, last_sec = -1,
    title = "",
}
local seek_zone = nil

local ov = mp.create_osd_overlay("ass-events")

local function fmt(t)
    t = math.max(0, math.floor(t or 0))
    local h, m, s = math.floor(t / 3600), math.floor(t % 3600 / 60), t % 60
    if h > 0 then return string.format("%d:%02d:%02d", h, m, s) end
    return string.format("%d:%02d", m, s)
end

-- ---------------------------------------------------------------------------
-- Rendu
-- ---------------------------------------------------------------------------
local function draw_button(i, b, x, y, h)
    local w = b.w * S
    local focused = st.focus == i
    if focused then
        gradient(x, y, w, h, h / 2)
    else
        local p = rrect(x, y, w, h, h / 2)
        shape(p, WHITE, 0.08)
        outline(p, 0.25)
    end
    zone(x, y, w, h, "button", i)
    local cx, cy = x + w / 2, y + h / 2
    if b.id == "pause" then
        if st.paused then
            shape(string.format("m %d %d l %d %d l %d %d", n(cx - 6 * S), n(cy - 9 * S), n(cx + 10 * S), n(cy),
                n(cx - 6 * S), n(cy + 9 * S)), WHITE, 1)
        else
            shape(rrect(cx - 8 * S, cy - 9 * S, 5 * S, 18 * S, 1.5 * S), WHITE, 1)
            shape(rrect(cx + 3 * S, cy - 9 * S, 5 * S, 18 * S, 1.5 * S), WHITE, 1)
        end
    else
        text(cx, cy, b.label, 19 * S, 5, { a = focused and 1 or 0.92, bold = focused })
    end
    return w
end

local function draw_panel(bottom)
    local p = st.panel
    local pw, rh = 460 * S, 44 * S
    local count = math.min(#p.entries, ROWS)
    local ph = 64 * S + math.max(1, count) * rh + 14 * S
    local px, py = W - 24 * S - pw, bottom - ph
    glass(px, py, pw, ph)
    zone(px, py, pw, ph, "panel")
    text(px + 24 * S, py + 34 * S, p.kind == "audio" and "Piste audio" or "Sous-titres", 24 * S, 4, { bold = true })
    if #p.entries > ROWS then
        text(px + pw - 24 * S, py + 34 * S, string.format("%d / %d", p.sel, #p.entries), 17 * S, 6, { a = 0.6 })
    end
    if #p.entries == 0 then
        text(px + 24 * S, py + 64 * S + rh / 2, "Aucune piste", 19 * S, 4, { a = 0.7 })
        return
    end
    local rx, rw = px + 12 * S, pw - 24 * S
    for i = p.top, math.min(#p.entries, p.top + ROWS - 1) do
        local e = p.entries[i]
        local ry = py + 60 * S + (i - p.top) * rh
        local sel = i == p.sel
        if sel then
            gradient(rx, ry + 3 * S, rw, rh - 6 * S, 10 * S)
        end
        if e.current then
            local r = 5 * S
            shape(rrect(rx + 18 * S - r, ry + rh / 2 - r, 2 * r, 2 * r, r), WHITE, 1)
        end
        text(rx + 38 * S, ry + rh / 2, e.label, 19 * S, 4,
            { a = sel and 1 or 0.85, bold = sel, clip = { rx, ry, rx + rw - 12 * S, ry + rh } })
        zone(rx, ry, rw, rh, "row", i)
    end
end

-- Sous-titres remontés au-dessus de la barre tant qu'elle est affichée (unités : 720 de haut).
local SUB_MARGIN = mp.get_property_number("sub-margin-y", 22)
local subs_raised = false
local function raise_subs(on)
    if on ~= subs_raised then
        subs_raised = on
        mp.set_property_number("sub-margin-y", on and SUB_MARGIN + 160 or SUB_MARGIN)
    end
end

local function render()
    out, hit, seek_zone = {}, {}, nil
    raise_subs(st.visible)
    if not st.visible then
        ov:remove()
        return
    end
    local m, bh = 24 * S, 128 * S
    local bx, by, bw = m, H - m - bh, W - 2 * m
    glass(bx, by, bw, bh)
    zone(bx, by, bw, bh, "bar")

    -- Titre et temps
    local time_txt = st.dur > 0 and (fmt(st.pos) .. " / " .. fmt(st.dur)) or fmt(st.pos)
    text(bx + 24 * S, by + 26 * S, st.title, 22 * S, 4,
        { bold = true, clip = { bx, by, bx + bw - 220 * S, by + 50 * S } })
    text(bx + bw - 24 * S, by + 26 * S, time_txt, 20 * S, 6, { a = 0.8 })

    -- Barre de progression
    local sx, sy, sw, sh = bx + 24 * S, by + 50 * S, bw - 48 * S, 6 * S
    shape(rrect(sx, sy, sw, sh, sh / 2), WHITE, 0.2)
    if st.dur > 0 then
        local frac = math.max(0, math.min(1, st.pos / st.dur))
        if frac > 0 then gradient(sx, sy, math.max(sh, sw * frac), sh, sh / 2) end
        local r, kx = 7 * S, sx + sw * frac
        shape(rrect(kx - r, sy + sh / 2 - r, 2 * r, 2 * r, r), WHITE, 1)
        seek_zone = { x = sx, w = sw }
        zone(sx, sy - 10 * S, sw, sh + 20 * S, "seek")
    end

    -- Boutons : groupe gauche (lecture), groupe droit (pistes, arrêt)
    local y, h, gap = by + 72 * S, 42 * S, 12 * S
    local x = bx + 24 * S
    local right_w = 0
    for _, b in ipairs(BUTTONS) do
        if b.right then right_w = right_w + b.w * S + gap end
    end
    local rx = bx + bw - 24 * S - (right_w - gap)
    for i, b in ipairs(BUTTONS) do
        if b.right then
            rx = rx + draw_button(i, b, rx, y, h) + gap
        else
            x = x + draw_button(i, b, x, y, h) + gap
        end
    end

    if st.panel then draw_panel(by - 16 * S) end

    ov.res_x, ov.res_y = W, H
    ov.data = table.concat(out, "\n")
    ov:update()
end

-- ---------------------------------------------------------------------------
-- Affichage / masquage
-- ---------------------------------------------------------------------------
local hide_timer
local function arm()
    hide_timer:kill()
    hide_timer:resume()
end
hide_timer = mp.add_timeout(HIDE_AFTER, function()
    if st.paused or st.panel or (st.hover and st.over_ui) then
        arm()
        return
    end
    st.visible, st.focus = false, nil
    render()
end)
hide_timer:kill()

local function show()
    st.visible = true
    arm()
    render()
end

local function hide()
    st.visible, st.focus, st.panel = false, nil, nil
    render()
end

-- ---------------------------------------------------------------------------
-- Pistes
-- ---------------------------------------------------------------------------
local function build_entries(kind)
    local list = mp.get_property_native("track-list") or {}
    local entries = {}
    if kind == "sub" then entries[1] = { id = nil, label = "Désactivés", current = false } end
    for _, t in ipairs(list) do
        if t.type == kind then
            local parts = {}
            if t.lang and t.lang ~= "" then parts[#parts + 1] = t.lang:upper() end
            -- Sous-titres externes : le « titre » est souvent un bout d'URL, inutile à afficher.
            if t.title and t.title ~= "" and not t.title:find("[/?]") then parts[#parts + 1] = t.title end
            if t.codec and t.codec ~= "" then parts[#parts + 1] = t.codec:upper() end
            if kind == "audio" and t["demux-channel-count"] then
                parts[#parts + 1] = string.format("%d canaux", t["demux-channel-count"])
            end
            if t.external then parts[#parts + 1] = "externe" end
            entries[#entries + 1] = {
                id = t.id,
                label = #parts > 0 and table.concat(parts, " · ") or ("Piste " .. t.id),
                current = t.selected and true or false,
            }
        end
    end
    if kind == "sub" then
        local any = false
        for i = 2, #entries do
            if entries[i].current then any = true end
        end
        entries[1].current = not any
    end
    return entries
end

local function ensure_visible()
    local p = st.panel
    if p.sel < p.top then p.top = p.sel end
    if p.sel > p.top + ROWS - 1 then p.top = p.sel - ROWS + 1 end
end

local function open_panel(kind)
    if st.panel and st.panel.kind == kind then
        st.panel = nil
    else
        local entries = build_entries(kind)
        local sel = 1
        for i, e in ipairs(entries) do
            if e.current then sel = i end
        end
        st.panel = { kind = kind, entries = entries, sel = sel, top = 1 }
        ensure_visible()
    end
    show()
end

local function move_sel(d)
    local p = st.panel
    if #p.entries == 0 then return end
    p.sel = math.max(1, math.min(#p.entries, p.sel + d))
    ensure_visible()
end

local function apply_sel()
    local p = st.panel
    local e = p.entries[p.sel]
    if e then
        mp.set_property_native(p.kind == "audio" and "aid" or "sid", e.id or "no")
    end
    st.panel = nil
end

-- ---------------------------------------------------------------------------
-- Actions
-- ---------------------------------------------------------------------------
local function volume(d)
    mp.command("no-osd add volume " .. d)
    mp.osd_message(string.format("Volume : %d %%", mp.get_property_number("volume", 0)), 1.2)
end

local function activate(id)
    if id == "pause" then
        mp.command("cycle pause")
    elseif id == "back" then
        mp.command("no-osd seek -10 relative")
    elseif id == "fwd" then
        mp.command("no-osd seek 10 relative")
    elseif id == "audio" or id == "sub" then
        open_panel(id)
        return
    elseif id == "stop" then
        mp.command("quit")
        return
    end
    show()
end

local function handle(k)
    if st.panel then
        if k == "up" then
            move_sel(-1)
        elseif k == "down" then
            move_sel(1)
        elseif k == "enter" or k == "space" then
            apply_sel()
        elseif k == "escape" or k == "left" then
            st.panel = nil
        elseif k == "audio" or k == "sub" then
            open_panel(k)
            return
        end
        show()
        return
    end
    if k == "audio" or k == "sub" then
        open_panel(k)
        return
    end
    if st.visible and st.focus then
        -- Navigation dans les boutons
        if k == "left" then
            st.focus = math.max(1, st.focus - 1)
        elseif k == "right" then
            st.focus = math.min(#BUTTONS, st.focus + 1)
        elseif k == "enter" then
            activate(BUTTONS[st.focus].id)
            return
        elseif k == "space" then
            mp.command("cycle pause")
        elseif k == "up" then
            volume(5)
        elseif k == "down" then
            volume(-5)
        elseif k == "escape" then
            hide()
            return
        end
        show()
        return
    end
    -- Barre masquée, ou affichée sans bouton sélectionné
    if k == "enter" then
        st.focus = 1
    elseif k == "space" then
        mp.command("cycle pause")
    elseif k == "left" then
        mp.command("no-osd seek -10 relative")
    elseif k == "right" then
        mp.command("no-osd seek 10 relative")
    elseif k == "up" then
        volume(5)
    elseif k == "down" then
        volume(-5)
    elseif k == "escape" then
        if st.visible then hide() else mp.command("quit") end
        return
    end
    show()
end

-- ---------------------------------------------------------------------------
-- Souris
-- ---------------------------------------------------------------------------
local function hit_at(x, y)
    for i = #hit, 1, -1 do
        local z = hit[i]
        if x >= z.x and x <= z.x + z.w and y >= z.y and y <= z.y + z.h then return z end
    end
end

mp.observe_property("mouse-pos", "native", function(_, m)
    if not m then return end
    -- mpv signale parfois la souris sans qu'elle ait bougé : on ne réagit qu'à un vrai déplacement,
    -- sinon la sélection faite au clavier / à la télécommande serait effacée.
    local moved = m.x ~= st.mx or m.y ~= st.my
    st.mx, st.my, st.hover = m.x, m.y, m.hover
    if not m.hover then
        st.over_ui = false
        return
    end
    if not moved then return end
    if not st.visible then
        show()
    end
    local z = hit_at(st.mx, st.my)
    st.over_ui = z ~= nil
    if z and z.act == "button" then
        st.focus = z.idx
    elseif z and z.act == "row" then
        st.panel.sel = z.idx
    elseif not z or z.act == "bar" then
        st.focus = nil
    end
    show()
end)

local function click()
    local z = hit_at(st.mx, st.my)
    if not z then
        if st.panel then
            st.panel = nil
            show()
        else
            mp.command("cycle pause")
        end
        return
    end
    if z.act == "button" then
        activate(BUTTONS[z.idx].id)
    elseif z.act == "row" then
        st.panel.sel = z.idx
        apply_sel()
        show()
    elseif z.act == "seek" and seek_zone then
        local pct = math.max(0, math.min(100, (st.mx - seek_zone.x) / seek_zone.w * 100))
        mp.command(string.format("no-osd seek %.3f absolute-percent", pct))
        show()
    end
end

local function wheel(d)
    if st.panel then
        move_sel(d)
        show()
    else
        volume(-5 * d)
    end
end

mp.add_forced_key_binding("MBTN_LEFT", "tf_click", click)
mp.add_forced_key_binding("WHEEL_UP", "tf_wheel_up", function() wheel(-1) end)
mp.add_forced_key_binding("WHEEL_DOWN", "tf_wheel_down", function() wheel(1) end)

-- ---------------------------------------------------------------------------
-- Clavier : via Turtlefin (IPC) ou directement dans mpv
-- ---------------------------------------------------------------------------
mp.register_script_message("key", handle)

local KEYS = {
    UP = "up", DOWN = "down", LEFT = "left", RIGHT = "right",
    ENTER = "enter", KP_ENTER = "enter", SPACE = "space",
    ESC = "escape", BS = "escape", a = "audio", s = "sub",
}
for key, name in pairs(KEYS) do
    mp.add_forced_key_binding(key, "tf_key_" .. key, function() handle(name) end,
        { repeatable = name == "up" or name == "down" or name == "left" or name == "right" })
end

-- ---------------------------------------------------------------------------
-- Propriétés observées
-- ---------------------------------------------------------------------------
mp.observe_property("osd-dimensions", "native", function(_, d)
    if d and d.w and d.w > 0 and d.h > 0 then
        W, H = d.w, d.h
        S = H / 720
        render()
    end
end)

mp.observe_property("pause", "bool", function(_, v)
    st.paused = v or false
    if st.paused then show() else render() end
end)

mp.observe_property("time-pos", "number", function(_, v)
    st.pos = v or 0
    local sec = math.floor(st.pos)
    if st.visible and sec ~= st.last_sec then
        st.last_sec = sec
        render()
    end
end)

mp.observe_property("duration", "number", function(_, v)
    st.dur = v or 0
end)

mp.observe_property("media-title", "string", function(_, v)
    st.title = v or ""
end)

-- Piste changée ailleurs (touche mpv, fin de chargement d'un sous-titre externe) : menu à jour.
mp.observe_property("track-list", "native", function()
    if st.panel then
        local sel = st.panel.sel
        st.panel.entries = build_entries(st.panel.kind)
        st.panel.sel = math.max(1, math.min(#st.panel.entries, sel))
        ensure_visible()
        render()
    end
end)

mp.register_event("file-loaded", show)
