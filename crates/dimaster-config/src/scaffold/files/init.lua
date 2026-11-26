-- Точка входа настроек DimaSter. Обычно менять этот файл не нужно.
--
-- Настройки складываются из слоёв: каждый следующий перекрывает предыдущий.
-- Порядок слоёв описан в lib/layers.lua, правила объединения — в lib/merge.lua.

local layers = require("lib.layers")
local merge = require("lib.merge")

return merge.all(layers.load(dimaster))
