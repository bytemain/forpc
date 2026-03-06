const { Peer } = require('..')
const { encode, decode } = require('@msgpack/msgpack')

const url = process.argv[2] || 'tcp://127.0.0.1:24010'
const method = process.argv[3] || 'MsgPack/Echo'
const msg = process.argv[4] || 'Hello'

;(async () => {
  const p = await Peer.connect(url)
  const resp = await p.callRaw(method, Buffer.from(encode({ message: msg })))
  const obj = decode(resp)
  process.stdout.write(`reply: ${obj.message}\n`)
})().catch((e) => {
  console.error(e)
  process.exit(1)
})
