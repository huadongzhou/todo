export function getDateTime (time) {
  let year, month, day, hour, minute, second
  let date = new Date()
  year = date.getFullYear()
  month = date.getMonth() + 1
  day = date.getDate()
  month < 10 && (month = '0' + month)
  day < 10 && (day = '0' + day)
  if (time) {
    hour = date.getHours()
    minute = date.getMinutes()
    second = date.getSeconds()
    hour < 10 && (hour = '0' + hour)
    minute < 10 && (minute = '0' + minute)
    second < 10 && (second = '0' + second)
  }
  date = year + '/' + month + '/' + day
  return time ? date + ' ' + hour + ':' + minute + ':' + second : date
}
export function getDateStr (date) {
  const day = ['今天', '昨天', '前天']
  let oldTime = new Date(date).getTime()
  let newTime = new Date(getDateTime()).getTime()
  let days = (newTime - oldTime) / 86400000
  if (days >= 0 && days < 8) {
    return day[days] ? day[days] : days + ' 天前'
  } else {
    return date
  }
}
/**
 * This is just a simple version of deep copy
 * Has a lot of edge cases bug
 * If you want to use a perfect deep copy, use lodash's _.cloneDeep
 * @param {Object} source
 * @returns {Object}
 */
export function deepClone (source) {
  if (!source && typeof source !== 'object') {
    throw new Error('error arguments', 'deepClone')
  }
  const targetObj = source.constructor === Array ? [] : {}
  Object.keys(source).forEach(keys => {
    if (source[keys] && typeof source[keys] === 'object') {
      targetObj[keys] = deepClone(source[keys])
    } else {
      targetObj[keys] = source[keys]
    }
  })
  return targetObj
}

