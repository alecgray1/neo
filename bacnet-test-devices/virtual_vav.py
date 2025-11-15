#!/usr/bin/env python3
"""
virtual_vav.py - Simulates 5 Variable Air Volume (VAV) BACnet devices

Each VAV has:
- AI:1 - Zone Temperature Sensor
- AO:1 - Damper Position
- BI:1 - Occupancy Sensor
- AV:1 - Temperature Setpoint
"""
import sys
import asyncio
import random
from bacpypes3.debugging import bacpypes_debugging, ModuleLogger
from bacpypes3.argparse import SimpleArgumentParser
from bacpypes3.app import Application
from bacpypes3.local.device import DeviceObject
from bacpypes3.local.analog import AnalogValueObject, AnalogInputObject, AnalogOutputObject
from bacpypes3.local.binary import BinaryInputObject
from bacpypes3.primitivedata import Real
from bacpypes3.basetypes import EngineeringUnits

# Module logger
_debug = 0
_log = ModuleLogger(globals())


@bacpypes_debugging
class VirtualVAVApplication:
    """Application managing multiple virtual VAV devices"""

    def __init__(self):
        """Initialize the VAV application"""
        self.devices = []
        self.running = True

    async def create_vav_device(self, device_instance: int, device_name: str, address: str):
        """Create a single VAV device with all its objects"""

        # Create the device object
        device_object = DeviceObject(
            objectName=device_name,
            objectIdentifier=("device", device_instance),
            maxApduLengthAccepted=1024,
            segmentationSupported="segmentedBoth",
            vendorIdentifier=15,
            vendorName="Neo Virtual Device",
            modelName="Virtual VAV Unit",
        )

        # Create the application for this device
        app = Application(device_object, address)

        # Add Zone Temperature Sensor (AI:1)
        temp_sensor = AnalogInputObject(
            objectIdentifier=("analogInput", 1),
            objectName=f"{device_name}-Temperature",
            presentValue=Real(72.0 + (device_instance % 5)),
            units=EngineeringUnits.degreesFahrenheit,
            description="Zone temperature sensor",
        )
        app.add_object(temp_sensor)

        # Add Damper Position (AO:1)
        damper = AnalogOutputObject(
            objectIdentifier=("analogOutput", 1),
            objectName=f"{device_name}-Damper",
            presentValue=Real(45.0),
            units=EngineeringUnits.percent,
            description="Damper position control",
        )
        app.add_object(damper)

        # Add Occupancy Sensor (BI:1)
        occupancy = BinaryInputObject(
            objectIdentifier=("binaryInput", 1),
            objectName=f"{device_name}-Occupancy",
            presentValue="inactive" if device_instance % 2 == 0 else "active",
            description="Occupancy sensor",
        )
        app.add_object(occupancy)

        # Add Temperature Setpoint (AV:1)
        setpoint = AnalogValueObject(
            objectIdentifier=("analogValue", 1),
            objectName=f"{device_name}-Setpoint",
            presentValue=Real(72.0),
            units=EngineeringUnits.degreesFahrenheit,
            description="Temperature setpoint",
        )
        app.add_object(setpoint)

        self.devices.append({
            'app': app,
            'name': device_name,
            'instance': device_instance,
            'objects': {
                'temp': temp_sensor,
                'damper': damper,
                'occupancy': occupancy,
                'setpoint': setpoint,
            }
        })

        print(f"✓ Created {device_name} (instance {device_instance}) at {address}")
        print(f"  - AI:1 Zone Temperature: {temp_sensor.presentValue}°F")
        print(f"  - AO:1 Damper Position: {damper.presentValue}%")
        print(f"  - BI:1 Occupancy: {occupancy.presentValue}")
        print(f"  - AV:1 Setpoint: {setpoint.presentValue}°F")

        return app

    async def simulate_values(self):
        """Periodically update values to simulate real device behavior"""
        while self.running:
            # Random sleep between 5-10 seconds
            sleep_duration = random.randint(5, 10)
            await asyncio.sleep(sleep_duration)

            for device in self.devices:
                # Simulate temperature drift
                temp = device['objects']['temp']
                current_temp = float(temp.presentValue)
                new_temp = current_temp + random.uniform(-0.5, 0.5)
                new_temp = max(65.0, min(80.0, new_temp))  # Clamp to realistic range
                temp.presentValue = Real(new_temp)

                # Simulate damper adjustment
                damper = device['objects']['damper']
                current_damper = float(damper.presentValue)
                new_damper = current_damper + random.uniform(-5.0, 5.0)
                new_damper = max(0.0, min(100.0, new_damper))  # Clamp 0-100%
                damper.presentValue = Real(new_damper)

                # Occasionally toggle occupancy
                if random.random() < 0.1:  # 10% chance each cycle
                    occupancy = device['objects']['occupancy']
                    occupancy.presentValue = "active" if occupancy.presentValue == "inactive" else "inactive"

                if _debug:
                    print(f"📊 {device['name']}: Temp={new_temp:.1f}°F, Damper={new_damper:.1f}%, Occ={device['objects']['occupancy'].presentValue}")


async def main():
    """Main entry point"""
    print("=" * 60)
    print("Virtual VAV BACnet Devices")
    print("=" * 60)
    print()

    # Create the application
    vav_app = VirtualVAVApplication()

    # Create 5 VAV devices
    # Using different ports on localhost
    base_instance = 101
    for i in range(5):
        device_instance = base_instance + i
        device_name = f"VAV-{i+1}"
        # Each device gets its own port
        port = 47808 + i
        address = f"127.0.0.1:{port}"

        await vav_app.create_vav_device(device_instance, device_name, address)

    print()
    print("=" * 60)
    print("All VAV devices are running!")
    print("Press Ctrl+C to stop...")
    print("=" * 60)
    print()

    # Start value simulation task
    simulation_task = asyncio.create_task(vav_app.simulate_values())

    try:
        # Run forever
        await asyncio.Event().wait()
    except KeyboardInterrupt:
        print("\n\nShutting down VAV devices...")
        vav_app.running = False
        simulation_task.cancel()
        try:
            await simulation_task
        except asyncio.CancelledError:
            pass


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        pass
