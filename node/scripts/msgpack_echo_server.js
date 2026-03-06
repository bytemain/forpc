const { RawServer } = require('..')
const { encode, decode } = require('@msgpack/msgpack')

const url = process.argv[2] || 'tcp://127.0.0.1:24010'
const method = process.argv[3] || 'MsgPack/Echo'

RawServer.listen(url, method, (payload) => {
  const obj = decode(payload)
  return Buffer.from(encode({ message: obj.message }))
})

setInterval(() => {}, 1 << 30)
