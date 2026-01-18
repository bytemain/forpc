const { Peer } = require('..')

const url = process.argv[2] || 'tcp://127.0.0.1:24002'
const method = process.argv[3] || 'Raw/Echo'
const msg = process.argv[4] || 'Hello'

;(async () => {
  const p = await Peer.connect(url)
  const resp = await p.callRaw(method, Buffer.from(msg))
  process.stdout.write(`reply: ${resp.toString()}\n`)
})().catch((e) => {
  console.error(e)
  process.exit(1)
})

