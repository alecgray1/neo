#!/usr/bin/env python3
"""
Simple BACnet device for Docker deployment
Accepts device configuration via environment variables
"""
import asyncio
import sys
import os
import random
from bacpypes3.argparse import SimpleArgumentParser
from bacpypes3.app import Application
from bacpypes3.local.device import DeviceObject
from bacpypes3.local.analog import AnalogInputObject, AnalogOutputObject, AnalogValueObject
from bacpypes3.local.binary import BinaryInputObject
from bacpypes3.primitivedata import Real
from bacpypes3.basetypes import EngineeringUnits


async def simulate_values(device_type, objects):
    """Periodically update values to simulate real device behavior"""
    while True:
        # Random sleep between 5-10 seconds
        sleep_duration = random.randint(5, 10)
        await asyncio.sleep(sleep_duration)

        if device_type == "VAV":
            # Simulate temperature drift
            temp = objects.get('temp')
            if temp:
                current_temp = float(temp.presentValue)
                new_temp = current_temp + random.uniform(-0.5, 0.5)
                new_temp = max(65.0, min(80.0, new_temp))
                temp.presentValue = Real(new_temp)

            # Simulate damper adjustment
            damper = objects.get('damper')
            if damper:
                current_damper = float(damper.presentValue)
                new_damper = current_damper + random.uniform(-5.0, 5.0)
                new_damper = max(0.0, min(100.0, new_damper))
                damper.presentValue = Real(new_damper)

            # Occasionally toggle occupancy
            if random.random() < 0.1:
                occupancy = objects.get('occupancy')
                if occupancy:
                    occupancy.presentValue = "active" if occupancy.presentValue == "inactive" else "inactive"

        elif device_type == "AHU":
            # Simulate supply temperature variation
            supply = objects.get('supply_temp')
            if supply:
                current_supply = float(supply.presentValue)
                new_supply = current_supply + random.uniform(-1.0, 1.0)
                new_supply = max(50.0, min(60.0, new_supply))
                supply.presentValue = Real(new_supply)

            # Simulate return temperature variation
            return_temp = objects.get('return_temp')
            if return_temp:
                current_return = float(return_temp.presentValue)
                new_return = current_return + random.uniform(-0.5, 0.5)
                new_return = max(68.0, min(76.0, new_return))
                return_temp.presentValue = Real(new_return)

            # Simulate fan speed adjustment
            fan_speed = objects.get('fan_speed')
            if fan_speed and return_temp:
                target_speed = 50.0 + (float(return_temp.presentValue) - 72.0) * 5.0
                current_speed = float(fan_speed.presentValue)
                new_speed = current_speed + (target_speed - current_speed) * 0.1
                new_speed = max(30.0, min(100.0, new_speed))
                fan_speed.presentValue = Real(new_speed)


async def main():
    # Get configuration from environment variables or use defaults
    device_name = os.getenv("DEVICE_NAME", "Device-1")
    device_instance = int(os.getenv("DEVICE_INSTANCE", "101"))
    device_type = os.getenv("DEVICE_TYPE", "VAV")
    ip_address = os.getenv("IP_ADDRESS", "0.0.0.0")

    print(f"Starting BACnet {device_type}: {device_name} (instance {device_instance}) on {ip_address}:47808")

    # Use BACpypes3 argument parser to properly initialize
    parser = SimpleArgumentParser()
    args = parser.parse_args([
        "--name", device_name,
        "--instance", str(device_instance),
        "--address", f"{ip_address}/24:47808"
    ])

    # Create application using from_args for proper initialization
    app = Application.from_args(args)

    # Store objects for simulation
    objects = {}

    if device_type == "VAV":
        # VAV devices: AI:1 (temp), AO:1 (damper), BI:1 (occupancy), AV:1 (setpoint)
        temp_sensor = AnalogInputObject(
            objectIdentifier=("analogInput", 1),
            objectName=f"{device_name}-Temperature",
            presentValue=Real(73.0),
            units=EngineeringUnits.degreesFahrenheit,
            description="Zone temperature sensor",
        )
        app.add_object(temp_sensor)
        objects['temp'] = temp_sensor

        damper = AnalogOutputObject(
            objectIdentifier=("analogOutput", 1),
            objectName=f"{device_name}-Damper",
            presentValue=Real(45.0),
            units=EngineeringUnits.percent,
            description="Damper position control",
        )
        app.add_object(damper)
        objects['damper'] = damper

        occupancy = BinaryInputObject(
            objectIdentifier=("binaryInput", 1),
            objectName=f"{device_name}-Occupancy",
            presentValue="active",
            description="Occupancy sensor",
        )
        app.add_object(occupancy)
        objects['occupancy'] = occupancy

        setpoint = AnalogValueObject(
            objectIdentifier=("analogValue", 1),
            objectName=f"{device_name}-Setpoint",
            presentValue=Real(72.0),
            units=EngineeringUnits.degreesFahrenheit,
            description="Temperature setpoint",
        )
        app.add_object(setpoint)
        objects['setpoint'] = setpoint

        print(f"✓ Created VAV with 4 objects: AI:1, AO:1, BI:1, AV:1")

    elif device_type == "AHU":
        # AHU devices: AI:1 (supply temp), AI:2 (return temp), BV:1 (fan status), AO:1 (fan speed)
        supply_temp = AnalogInputObject(
            objectIdentifier=("analogInput", 1),
            objectName=f"{device_name}-SupplyTemp",
            presentValue=Real(55.0),
            units=EngineeringUnits.degreesFahrenheit,
            description="Supply air temperature",
        )
        app.add_object(supply_temp)
        objects['supply_temp'] = supply_temp

        return_temp = AnalogInputObject(
            objectIdentifier=("analogInput", 2),
            objectName=f"{device_name}-ReturnTemp",
            presentValue=Real(72.0),
            units=EngineeringUnits.degreesFahrenheit,
            description="Return air temperature",
        )
        app.add_object(return_temp)
        objects['return_temp'] = return_temp

        fan_speed = AnalogOutputObject(
            objectIdentifier=("analogOutput", 1),
            objectName=f"{device_name}-FanSpeed",
            presentValue=Real(75.0),
            units=EngineeringUnits.percent,
            description="Fan speed control",
        )
        app.add_object(fan_speed)
        objects['fan_speed'] = fan_speed

        print(f"✓ Created AHU with 3 objects: AI:1, AI:2, AO:1")

    print(f"✓ {device_name} is ready and listening for BACnet requests")
    print(f"  Device Instance: {device_instance}")
    print(f"  Listening on: {ip_address}:47808")
    print(f"  Value simulation: enabled (updates every 5-10 seconds)")
    sys.stdout.flush()

    # Start value simulation task
    simulation_task = asyncio.create_task(simulate_values(device_type, objects))

    try:
        # Run forever
        await asyncio.Future()
    except asyncio.CancelledError:
        simulation_task.cancel()
        try:
            await simulation_task
        except asyncio.CancelledError:
            pass


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print(f"\nShutting down...")
        sys.exit(0)
