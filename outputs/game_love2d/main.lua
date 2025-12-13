lua
-- Main game file for a simple space shooter game
-- This game features a player-controlled ship that shoots at incoming enemies

-- Global variables
local player = {
    x = 400,
    y = 550,
    width = 50,
    height = 30,
    speed = 300,
    bullets = {},
    bulletSpeed = 500,
    cooldown = 0.2,
    lastShot = 0
}

local enemies = {}
local enemySpawnTimer = 0
local enemySpawnRate = 1.0
local score = 0
local gameState = "start" -- "start", "playing", "gameover"
local gameFont = nil
local largeFont = nil

-- Load game resources and initialize
function love.load()
    -- Set random seed
    math.randomseed(os.time())
    
    -- Load fonts
    gameFont = love.graphics.newFont(14)
    largeFont = love.graphics.newFont(32)
    
    -- Set default filter for scaling images
    love.graphics.setDefaultFilter("nearest", "nearest")
    
    -- Set window title
    love.window.setTitle("Space Shooter")
end

-- Update game state
function love.update(dt)
    if gameState == "playing" then
        -- Player movement
        if love.keyboard.isDown("left") or love.keyboard.isDown("a") then
            player.x = math.max(player.x - player.speed * dt, 0)
        end
        if love.keyboard.isDown("right") or love.keyboard.isDown("d") then
            player.x = math.min(player.x + player.speed * dt, love.graphics.getWidth() - player.width)
        end
        
        -- Shooting
        if love.keyboard.isDown("space") and player.lastShot > player.cooldown then
            local bullet = {
                x = player.x + player.width / 2 - 2,
                y = player.y,
                width = 4,
                height = 10
            }
            table.insert(player.bullets, bullet)
            player.lastShot = 0
        end
        player.lastShot = player.lastShot + dt
        
        -- Update bullets
        for i = #player.bullets, 1, -1 do
            local bullet = player.bullets[i]
            bullet.y = bullet.y - player.bulletSpeed * dt
            
            -- Remove bullets that go off screen
            if bullet.y < -bullet.height then
                table.remove(player.bullets, i)
            end
        end
        
        -- Spawn enemies
        enemySpawnTimer = enemySpawnTimer + dt
        if enemySpawnTimer > enemySpawnRate then
            local enemy = {
                x = math.random(0, love.graphics.getWidth() - 40),
                y = -40,
                width = 40,
                height = 40,
                speed = math.random(100, 200)
            }
            table.insert(enemies, enemy)
            enemySpawnTimer = 0
            
            -- Increase difficulty over time
            enemySpawnRate = math.max(0.3, enemySpawnRate - 0.01)
        end
        
        -- Update enemies
        for i = #enemies, 1, -1 do
            local enemy = enemies[i]
            enemy.y = enemy.y + enemy.speed * dt
            
            -- Check for collision with player
            if checkCollision(enemy, player) then
                gameState = "gameover"
                break
            end
            
            -- Check for collision with bullets
            for j = #player.bullets, 1, -1 do
                local bullet = player.bullets[j]
                if checkCollision(bullet, enemy) then
                    table.remove(enemies, i)
                    table.remove(player.bullets, j)
                    score = score + 10
                    break
                end
            end
            
            -- Remove enemies that go off screen
            if enemy.y > love.graphics.getHeight() then
                table.remove(enemies, i)
            end
        end
    end
end

-- Draw game elements
function love.draw()
    if gameState == "start" then
        -- Draw start screen
        love.graphics.setFont(largeFont)
        love.graphics.printf("SPACE SHOOTER", 0, 200, love.graphics.getWidth(), "center")
        love.graphics.setFont(gameFont)
        love.graphics.printf("Press ENTER to start", 0, 300, love.graphics.getWidth(), "center")
        love.graphics.printf("Use LEFT/RIGHT or A/D to move", 0, 350, love.graphics.getWidth(), "center")
        love.graphics.printf("Press SPACE to shoot", 0, 370, love.graphics.getWidth(), "center")
    elseif gameState == "playing" then
        -- Draw player
        love.graphics.setColor(0, 1, 1)
        love.graphics.rectangle("fill", player.x, player.y, player.width, player.height)
        
        -- Draw player bullets
        love.graphics.setColor(1, 1, 0)
        for _, bullet in ipairs(player.bullets) do
            love.graphics.rectangle("fill", bullet.x, bullet.y, bullet.width, bullet.height)
        end
        
        -- Draw enemies
        love.graphics.setColor(1, 0, 0)
        for _, enemy in ipairs(enemies) do
            love.graphics.rectangle("fill", enemy.x, enemy.y, enemy.width, enemy.height)
        end
        
        -- Draw score
        love.graphics.setColor(1, 1, 1)
        love.graphics.setFont(gameFont)
        love.graphics.print("Score: " .. score, 10, 10)
    elseif gameState == "gameover" then
        -- Draw game over screen
        love.graphics.setFont(largeFont)
        love.graphics.printf("GAME OVER", 0, 200, love.graphics.getWidth(), "center")
        love.graphics.setFont(gameFont)
        love.graphics.printf("Final Score: " .. score, 0, 300, love.graphics.getWidth(), "center")
        love.graphics.printf("Press ENTER to play again", 0, 350, love.graphics.getWidth(), "center")
    end
end

-- Handle key presses
function love.keypressed(key)
    if key == "escape" then
        love.event.quit()
    elseif gameState == "start" and (key == "return" or key == "kpenter") then
        resetGame()
        gameState = "playing"
    elseif gameState == "gameover" and (key == "return" or key == "kpenter") then
        resetGame()
        gameState = "playing"
    end
end

-- Reset game state
function resetGame()
    player.x = 400
    player.y = 550
    player.bullets = {}
    player.lastShot = 0
    
    enemies = {}
    enemySpawnTimer = 0
    enemySpawnRate = 1.0
    score = 0
end

-- Check collision between two rectangles
function checkCollision(a, b)
    return a.x < b.x + b.width and
           a.x + a.width > b.x and
           a.y < b.y + b.height and
           a.y + a.height > b.y
end