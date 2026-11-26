-- Пример плагина DimaSter. Каждый файл plugins/*.lua загружается при старте
-- и после изменения настроек.
--
-- События:  dimaster.on("startup" | "title" | "cwd" | "focus" | "notification", function(value) end)
-- Команды:  dimaster.command(имя, название_в_палитре, function() end)
--           клавиша: { key = "d", mods = "ctrl+shift", action = "plugin:insert_date" }
-- Действия: dimaster.send(текст), dimaster.action(имя), dimaster.notify(заголовок, текст),
--           dimaster.log(текст)
-- Чтение:   dimaster.title(), dimaster.cwd(), dimaster.selection(), dimaster.screen_text()

dimaster.command("insert_date", dimaster.tr("Вставить сегодняшнюю дату"), function()
  dimaster.send(os.date("%Y-%m-%d"))
end)
