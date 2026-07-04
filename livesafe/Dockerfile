FROM node:22-alpine AS client-build

WORKDIR /app/client

COPY client/package*.json ./
RUN npm ci

COPY client/ ./
RUN npm run build

FROM node:22-alpine

WORKDIR /app

COPY server/package*.json ./server/
RUN cd server && npm ci --omit=dev

COPY server/ ./server/
COPY config/ ./config/
COPY --from=client-build /app/client/dist ./client/dist/

EXPOSE 8080

ENV PORT=8080
ENV NODE_ENV=production

CMD ["node", "server/index.js"]
