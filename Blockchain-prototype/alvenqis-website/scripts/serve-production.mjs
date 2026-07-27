import { createReadStream, existsSync, statSync } from 'node:fs'
import { createServer } from 'node:http'
import { extname, join, normalize, resolve } from 'node:path'

const root = resolve(process.env.WEBSITE_DIST_DIR || 'dist')
const port = Number.parseInt(process.env.PORT || '18081', 10)
const host = process.env.HOST || '0.0.0.0'

if (!existsSync(join(root, 'index.html'))) {
  throw new Error(`Production website build is missing at ${root}; run npm run build first`)
}
if (!Number.isInteger(port) || port < 1 || port > 65535) {
  throw new Error(`Invalid PORT: ${process.env.PORT}`)
}

const contentTypes = {
  '.css': 'text/css; charset=utf-8',
  '.html': 'text/html; charset=utf-8',
  '.ico': 'image/x-icon',
  '.js': 'text/javascript; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.png': 'image/png',
  '.svg': 'image/svg+xml',
  '.webp': 'image/webp',
  '.woff': 'font/woff',
  '.woff2': 'font/woff2',
}

function safeAssetPath(pathname) {
  const decoded = decodeURIComponent(pathname).replaceAll('\\', '/')
  const relative = normalize(decoded).replace(/^([/\\])+/, '')
  const candidate = resolve(root, relative)
  return candidate === root || candidate.startsWith(`${root}\\`) || candidate.startsWith(`${root}/`)
    ? candidate
    : null
}

const server = createServer((request, response) => {
  response.setHeader('X-Content-Type-Options', 'nosniff')
  response.setHeader('Referrer-Policy', 'strict-origin-when-cross-origin')
  response.setHeader('X-Frame-Options', 'DENY')

  if (request.url === '/healthz') {
    response.writeHead(200, { 'Content-Type': 'application/json; charset=utf-8', 'Cache-Control': 'no-store' })
    response.end(JSON.stringify({ status: 'ok', service: 'alvenqis-website' }))
    return
  }

  const pathname = new URL(request.url || '/', 'http://localhost').pathname
  const asset = safeAssetPath(pathname)
  const file = asset && existsSync(asset) && statSync(asset).isFile() ? asset : join(root, 'index.html')
  const isHtml = extname(file).toLowerCase() === '.html'
  response.writeHead(200, {
    'Content-Type': contentTypes[extname(file).toLowerCase()] || 'application/octet-stream',
    'Cache-Control': isHtml ? 'no-cache' : 'public, max-age=31536000, immutable',
  })
  if (request.method === 'HEAD') {
    response.end()
    return
  }
  createReadStream(file).pipe(response)
})

server.listen(port, host, () => {
  console.log(`Alvenqis website listening on http://${host}:${port}`)
})
