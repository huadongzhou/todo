
import { ipcRenderer } from 'electron'
import lowdb from 'lowdb'
import lodashId from 'lodash-id'
import FileSync from 'lowdb/adapters/FileSync'
import path from "path"
import fs from "fs-extra"


const isDevelopment = process.env.NODE_ENV != 'production'
let db

const DB = {
  init () {
    return new Promise((resolve) => {
      ipcRenderer.invoke("getuserPath").then(userPath => {
        const DBPath = isDevelopment ? '/data-dev.json' : '/data.json'

        const adapter = new FileSync(path.join(userPath, DBPath))

        db = lowdb(adapter)

        db._.mixin(lodashId)

        resolve()
        console.log('init初始化')
      })
    })
  },
  initDB (storePath) {
    if (fs.pathExistsSync(storePath)) {
      fs.mkdirpSync(storePath)
    }

    const DBPath = isDevelopment ? '/data-dev.json' : '/data.json'

    const adapter = new FileSync(path.join(storePath, DBPath))

    db = lowdb(adapter)

    db._.mixin(lodashId)

    db.defaults({
      todoList: [],
      doneList: [],
      setting: {}
    }).write()
    //判断是否为第一次进入
    if (!this.has("settings.firstRun")) {
      this.set("settings.firstRun", true)
    }
  },
  has (key) {
    return db
      .read()
      .has(key)
      .value()
  },
  get (key) {
    return db
      .read()
      .get(key)
      .value()
  },
  set (key, value) {
    return db
      .read()
      .set(key, value)
      .write()
  },
  insert (key, value) {
    return db
      .read()
      .get(key)
      .insert(value)
      .write()
  },
  update (key, match, value) {
    return db
      .read()
      .get(key)
      .find(match)
      .assign(value)
      .write()
  },
  removeById (key, id) {
    return db
      .read()
      .get(key)
      .removeById(id)
      .write()
  },
  groupby (key, prop) {
    const d = db
      .read()
      .get(key)
      .sortBy(prop)
      .reverse()
      .groupBy(prop)
      .value()
    return d
  }
}

export default DB