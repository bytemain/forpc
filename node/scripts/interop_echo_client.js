const { Peer } = require('..')
const protobuf = require('protobufjs')
const path = require('path')

const url = process.argv[2] || 'tcp://127.0.0.1:24000'
const msg = process.argv[3] || 'Hello'

;(async () => {
  const root = await protobuf.load(path.join(__dirname, '../../proto/forpc_test.proto'))
  const EchoRequest = root.lookupType('forpc.test.EchoRequest')
  const EchoResponse = root.lookupType('forpc.test.EchoResponse')

  const p = await Peer.connect(url)
  const reqBuf = Buffer.from(EchoRequest.encode(EchoRequest.create({ data: msg })).finish())
  const respBuf = await p.callRaw('Test/Echo', reqBuf)
  const resp = EchoResponse.decode(respBuf)
  process.stdout.write(`reply: ${resp.result}\n`)
})().catch((e) => {
  console.error(e)
  process.exit(1)
})
