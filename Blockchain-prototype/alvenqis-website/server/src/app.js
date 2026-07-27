import cookieParser from 'cookie-parser'
import cors from 'cors'
import express from 'express'
import helmet from 'helmet'
import { errorHandler, notFoundHandler } from './middleware/error.js'
import { corsOrigin, globalRateLimiter } from './middleware/security.js'
import adminRoutes from './routes/adminRoutes.js'
import authRoutes from './routes/authRoutes.js'
import contentRoutes from './routes/contentRoutes.js'
import docsRoutes from './routes/docsRoutes.js'
import faqRoutes from './routes/faqRoutes.js'
import healthRoutes from './routes/healthRoutes.js'
import networkRoutes from './routes/networkRoutes.js'
import roadmapRoutes from './routes/roadmapRoutes.js'

const app = express()

app.disable('x-powered-by')
app.set('trust proxy', 1)
app.use(helmet({
  crossOriginEmbedderPolicy: false,
}))
app.use(cors({
  origin: corsOrigin,
  credentials: true,
  methods: ['GET', 'POST', 'PUT', 'DELETE', 'OPTIONS'],
  allowedHeaders: ['Content-Type', 'Authorization'],
}))
app.use(globalRateLimiter)
app.use(express.json({ limit: '1mb' }))
app.use(cookieParser())

app.use('/', healthRoutes)
app.use('/', docsRoutes)
app.use('/auth', authRoutes)
app.use('/', adminRoutes)
app.use('/', contentRoutes)
app.use('/', roadmapRoutes)
app.use('/', faqRoutes)
app.use('/', networkRoutes)

app.use(notFoundHandler)
app.use(errorHandler)

export default app
