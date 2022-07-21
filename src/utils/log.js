const log = require("electron-log")
const isDevelopment = process.env.NODE_ENV !== 'production'

export default function () {
  if (isDevelopment) {
    console.log(arguments)
  } else {
    log.info(arguments)
  }
}