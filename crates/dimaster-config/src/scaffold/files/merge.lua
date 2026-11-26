-- Объединение слоёв настроек.
--
-- Таблицы-словари объединяются рекурсивно, списки заменяются целиком.
-- Исключение — keys: сочетания из всех слоёв складываются, и более поздний
-- слой переназначает те же клавиши.

local merge = {}

local function is_list(value)
  return type(value) == "table" and (#value > 0 or next(value) == nil)
end

local function append(base, extra)
  local result = {}
  for _, item in ipairs(base or {}) do result[#result + 1] = item end
  for _, item in ipairs(extra) do result[#result + 1] = item end
  return result
end

function merge.into(target, layer)
  for key, value in pairs(layer) do
    if key == "keys" then
      target[key] = append(target[key], value)
    elseif type(value) == "table" and not is_list(value) then
      target[key] = merge.into(type(target[key]) == "table" and target[key] or {}, value)
    else
      target[key] = value
    end
  end
  return target
end

function merge.all(layers)
  local result = {}
  for _, layer in ipairs(layers) do merge.into(result, layer) end
  return result
end

return merge
