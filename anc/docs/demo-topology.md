# CDR Demo Topology

A single wiring topology that can demo all peripherals.

## All assignments

| GPIO Pin | Function | Assignment | Demo |
| :----: | :-: | :-------: | :-------: |
| 20 | DI | HC-SR04 Echo | Distance measurement |
| 21 | DQ | HC-SR04 Trig | Distance measurement |
| 13 | PWM Output | L298N EN_B | Propulsion |
| 26 | DQ | L298N IN_3 | Propulsion |
| 19 | DQ | L298N IN_4 | Propulsion |
| 02 | I2C SDA | INA219 and Controller SDA | Power Monitoring |
| 03 | I2C SCL | INA219 and Controller SCL | Power Monitoring |
| 01 | 3.3Vref | I2C Vcc Bus, TTL shifter ref | Power Monitoring |
| 02 | 5Vref | 5Vref | Auxiliary |
| 39 | GND | Logic and power GND ref | Auxiliary |
| 12 | PWM output | SG-90 position command | Camera Panning Servo |


## Auxiliary

## Pin assignment

| GPIO Pin | Function | Assignment |
| :----: | :-: | :-------: |
| 01 | 3.3Vref | TTL shifter ref |
| 02 | 5Vref | 5Vref |
| 39 | GND | Logic and power GND ref |

## Distance measurement Demo

### Pin assignment

| GPIO Pin | Function | Assignment |
| :----: | :-: | :-------: |
| 20 | DI | HC-SR04 Echo |
| 21 | DQ | HC-SR04 Trig |

## Propulsion Demo

### Pin assignment

| GPIO Pin | Function | Assignment |
| :----: | :-: | :-------: |
| 13 | PWM Output | L298N EN_B |
| 26 | DQ | L298N IN_3 |
| 19 | DQ | L298N IN_4 |

## Power Monitoring Demo

### Pin assignment

| GPIO Pin | Function | Assignment |
| :----: | :-: | :-------: |
| 02 | I2C SDA | INA219 and Controller SDA |
| 03 | I2C SCL | INA219 and Controller SCL |
| 01 | 3.3Vref | I2C Vcc Bus |

## Camera Panning Demo

### Pin assignment

| GPIO Pin | Function | Assignment |
| :----: | :-: | :-------: |
| 12 | PWM output | SG-90 position command |
