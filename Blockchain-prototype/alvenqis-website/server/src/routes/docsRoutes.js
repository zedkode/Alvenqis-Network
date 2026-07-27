import { Router } from 'express'
import swaggerUi from 'swagger-ui-express'
import { openApiDocument } from '../docs/openapi.js'

const router = Router()

router.get('/openapi.json', (req, res) => {
  res.json(openApiDocument)
})

router.use('/api/docs', swaggerUi.serve, swaggerUi.setup(openApiDocument, {
  explorer: true,
  customSiteTitle: 'Alvenqis Network API Docs',
}))

export default router
