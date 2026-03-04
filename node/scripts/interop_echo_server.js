const { RawServer } = require('..')
const protobuf = require('protobufjs')
const path = require('path')

const url = process.argv[2] || 'tcp://127.0.0.1:24000'

;(async () => {
  const root = await protobuf.load(path.join(__dirname, '../../proto/forpc_test.proto'))
  const EchoRequest = root.lookupType('forpc.test.EchoRequest')
  const EchoResponse = root.lookupType('forpc.test.EchoResponse')

  const server = await RawServer.bind(url)
  server.registerUnary('Test/Echo', EchoRequest, EchoResponse, (req) => {
    return { result: req.data }
  })
  server.serve()
})().catch((e) => {
  console.error(e)
  process.exit(1)
})

setInterval(() => {}, 1 << 30)
